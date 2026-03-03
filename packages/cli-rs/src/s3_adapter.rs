use std::io::Write;
use std::path::Path;

use anyhow::{bail, Context, Result};
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::types::ChecksumMode;
use aws_sdk_s3::{primitives::ByteStream, Client};
use tokio::io::AsyncReadExt;

use crate::compression::{compress_to_temp, decompress_to_file};
use crate::config::{CompressionKind, RuntimeConfig, COMPRESSION_PREFERENCE};

pub struct S3Adapter {
    client: Client,
    config: RuntimeConfig,
    runtime: tokio::runtime::Runtime,
}

impl S3Adapter {
    pub fn new(config: &RuntimeConfig) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build tokio runtime")?;

        let client = build_client(&runtime, config)?;
        Ok(Self {
            client,
            config: config.clone(),
            runtime,
        })
    }

    pub fn upload<F: FnMut(i64)>(
        &self,
        oid: &str,
        local_path: &str,
        mut callback: F,
    ) -> Result<()> {
        let compression = self.config.compression_kind()?;
        let remote_path = format!("{}{}", self.as_lfs_path(oid), compression.extension());

        let compressed = compress_to_temp(Path::new(local_path), compression)?;

        if let Ok(head) = self.runtime.block_on(
            self.client
                .head_object()
                .bucket(&self.config.bucket)
                .key(&remote_path)
                .checksum_mode(ChecksumMode::Enabled)
                .send(),
        ) {
            if let Some(remote_size) = head.content_length() {
                if remote_size != compressed.size {
                    bail!(
                        "Existing remote file has different size, local: {}, remote: {}",
                        compressed.size,
                        remote_size
                    );
                }
            }

            if let Some(remote_checksum) = head.checksum_crc32_c() {
                if remote_checksum != compressed.checksum_crc32c_b64 {
                    bail!(
                        "Existing remote file has different checksum, local: {}, remote: {}",
                        compressed.checksum_crc32c_b64,
                        remote_checksum
                    );
                }
            }

            return Ok(());
        }

        let body = self
            .runtime
            .block_on(ByteStream::from_path(compressed.temp.path()))?;

        self.runtime.block_on(
            self.client
                .put_object()
                .bucket(&self.config.bucket)
                .key(&remote_path)
                .body(body)
                .send(),
        )?;

        callback(compressed.size);

        if self.config.delete_other_versions {
            for alt in COMPRESSION_PREFERENCE {
                let alt_path = format!("{}{}", self.as_lfs_path(oid), alt.extension());
                if alt_path == remote_path {
                    continue;
                }
                if self.file_exists(&alt_path) {
                    let _ = self.runtime.block_on(
                        self.client
                            .delete_object()
                            .bucket(&self.config.bucket)
                            .key(&alt_path)
                            .send(),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn download<F: FnMut(i64)>(
        &self,
        oid: &str,
        local_path: &str,
        mut callback: F,
    ) -> Result<()> {
        let (remote_path, compression) = self.resolve_download_path(oid)?;

        let output = self.runtime.block_on(
            self.client
                .get_object()
                .bucket(&self.config.bucket)
                .key(&remote_path)
                .send(),
        )?;

        let mut async_reader = output.body.into_async_read();
        let mut temp = tempfile::NamedTempFile::new()?;
        let mut buf = [0u8; 256 * 1024];

        loop {
            let n = self
                .runtime
                .block_on(async { async_reader.read(&mut buf).await })?;
            if n == 0 {
                break;
            }
            temp.write_all(&buf[..n])?;
            callback(n as i64);
        }
        temp.flush()?;

        decompress_to_file(temp.path(), Path::new(local_path), compression)?;
        Ok(())
    }

    fn resolve_download_path(&self, oid: &str) -> Result<(String, CompressionKind)> {
        let base = self.as_lfs_path(oid);

        for kind in COMPRESSION_PREFERENCE {
            let candidate = format!("{}{}", base, kind.extension());
            if self.file_exists(&candidate) {
                return Ok((candidate, kind));
            }
        }

        bail!("No downloadable version of the file was found")
    }

    fn file_exists(&self, key: &str) -> bool {
        self.runtime
            .block_on(
                self.client
                    .head_object()
                    .bucket(&self.config.bucket)
                    .key(key)
                    .send(),
            )
            .is_ok()
    }

    fn as_lfs_path(&self, path: &str) -> String {
        if self.config.root_path.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.config.root_path, path)
        }
    }
}

fn build_client(runtime: &tokio::runtime::Runtime, config: &RuntimeConfig) -> Result<Client> {
    let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));

    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .profile_name(std::env::var("AWS_PROFILE").unwrap_or_default());

    if !config.access_key_id.is_empty() {
        loader = loader.credentials_provider(Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "s3-lfs",
        ));
    }

    let shared = runtime.block_on(loader.load());

    let mut builder = aws_sdk_s3::config::Builder::from(&shared)
        .region(Region::new(config.region.clone()))
        .force_path_style(config.use_path_style)
        .endpoint_url(config.endpoint.clone());

    if config.endpoint.contains("storage.googleapis.com") {
        builder = builder.request_checksum_calculation(
            aws_sdk_s3::config::RequestChecksumCalculation::WhenRequired,
        );
    }

    Ok(Client::from_conf(builder.build()))
}
