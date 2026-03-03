use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::config::{CompressionKind, RuntimeConfig, DEFAULT_COMPRESSION};

const CONFIG_DIR: &str = ".config/s3-lfs/profiles";
const FILE_NAME: &str = "credentials.json";

#[derive(Clone, Debug, Serialize)]
pub struct Profile {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
    pub root_path: String,
    pub compression: String,
    pub use_path_style: bool,
    pub delete_other_versions: bool,
}

#[derive(Debug, Deserialize)]
struct ProfileFile {
    #[serde(default)]
    access_key_id: String,
    #[serde(default)]
    secret_access_key: String,
    #[serde(default)]
    bucket: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    root_path: String,
    compression: Option<String>,
    #[serde(default)]
    use_path_style: bool,
    delete_other_versions: Option<bool>,
}

pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty() || slug.len() > 64 {
        bail!("invalid profile slug \"{}\"", slug);
    }

    let mut chars = slug.chars();
    let first = chars
        .next()
        .ok_or_else(|| anyhow::anyhow!("invalid profile slug \"{}\"", slug))?;
    if !first.is_ascii_alphanumeric() {
        bail!("invalid profile slug \"{}\"", slug);
    }

    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("invalid profile slug \"{}\"", slug);
    }

    Ok(())
}

pub fn validate_profile(profile: &Profile) -> Result<()> {
    if profile.bucket.is_empty() {
        bail!("bucket is required");
    }
    if profile.endpoint.is_empty() {
        bail!("endpoint is required");
    }
    if profile.region.is_empty() {
        bail!("region is required");
    }
    if (profile.access_key_id.is_empty()) != (profile.secret_access_key.is_empty()) {
        bail!("access key and secret key should either both be set or both be empty");
    }
    if CompressionKind::parse(&profile.compression).is_none() {
        bail!("invalid compression \"{}\"", profile.compression);
    }
    Ok(())
}

fn config_base_dir() -> io::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?;
    Ok(home.join(CONFIG_DIR))
}

fn profile_dir(slug: &str) -> io::Result<PathBuf> {
    validate_slug(slug).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    Ok(config_base_dir()?.join(slug))
}

fn profile_path(slug: &str) -> io::Result<PathBuf> {
    Ok(profile_dir(slug)?.join(FILE_NAME))
}

pub fn save(slug: &str, profile: &Profile) -> io::Result<()> {
    validate_slug(slug).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    validate_profile(profile)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

    let path = profile_path(slug)?;
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("invalid profile path"))?;
    fs::create_dir_all(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }

    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    let mut payload = serde_json::to_vec_pretty(profile)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    payload.push(b'\n');
    std::io::Write::write_all(&mut tmp, &payload)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    tmp.persist(path)
        .map_err(|e| io::Error::other(e.error.to_string()))?;

    Ok(())
}

pub fn load(slug: &str) -> io::Result<Profile> {
    let content = fs::read(profile_path(slug)?)?;
    let raw: ProfileFile = serde_json::from_slice(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let profile = Profile {
        access_key_id: raw.access_key_id,
        secret_access_key: raw.secret_access_key,
        bucket: raw.bucket,
        endpoint: raw.endpoint,
        region: raw.region,
        root_path: raw.root_path,
        compression: raw
            .compression
            .unwrap_or_else(|| DEFAULT_COMPRESSION.to_string()),
        use_path_style: raw.use_path_style,
        delete_other_versions: raw.delete_other_versions.unwrap_or(true),
    };

    validate_profile(&profile)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(profile)
}

pub fn list() -> io::Result<Vec<String>> {
    let base = config_base_dir()?;
    let entries = match fs::read_dir(base) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let mut out = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        if validate_slug(&slug).is_ok() {
            out.push(slug);
        }
    }
    out.sort();
    Ok(out)
}

pub fn delete(slug: &str) -> io::Result<()> {
    let dir = profile_dir(slug)?;
    if !dir.exists() {
        return Ok(());
    }
    fs::remove_dir_all(dir)
}

pub fn runtime_from_profile(slug: &str) -> io::Result<RuntimeConfig> {
    let profile = load(slug)?;
    Ok(runtime_from_profile_obj(slug, &profile))
}

pub fn runtime_from_profile_obj(slug: &str, profile: &Profile) -> RuntimeConfig {
    RuntimeConfig {
        profile: slug.to_string(),
        access_key_id: profile.access_key_id.clone(),
        secret_access_key: profile.secret_access_key.clone(),
        bucket: profile.bucket.clone(),
        endpoint: profile.endpoint.clone(),
        region: profile.region.clone(),
        root_path: profile.root_path.clone(),
        use_path_style: profile.use_path_style,
        delete_other_versions: profile.delete_other_versions,
        compression: profile.compression.clone(),
    }
}

pub fn profile_from_runtime(cfg: &RuntimeConfig) -> Profile {
    Profile {
        access_key_id: cfg.access_key_id.clone(),
        secret_access_key: cfg.secret_access_key.clone(),
        bucket: cfg.bucket.clone(),
        endpoint: cfg.endpoint.clone(),
        region: cfg.region.clone(),
        root_path: cfg.root_path.clone(),
        compression: cfg.compression.clone(),
        use_path_style: cfg.use_path_style,
        delete_other_versions: cfg.delete_other_versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn slug_validation() {
        assert!(validate_slug("dev").is_ok());
        assert!(validate_slug("prod-01").is_ok());
        assert!(validate_slug("team_1").is_ok());
        assert!(validate_slug("-bad").is_err());
        assert!(validate_slug("bad/path").is_err());
    }

    #[test]
    fn save_load_list_delete_cycle() {
        let home = tempfile::tempdir().expect("tempdir");
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home.path());
        struct Guard(Option<String>);
        impl Drop for Guard {
            fn drop(&mut self) {
                if let Some(home) = &self.0 {
                    std::env::set_var("HOME", home);
                } else {
                    std::env::remove_var("HOME");
                }
            }
        }
        let _guard = Guard(old_home);

        let profile = Profile {
            access_key_id: "ak".to_string(),
            secret_access_key: "sk".to_string(),
            bucket: "bucket-a".to_string(),
            endpoint: "http://127.0.0.1:9000".to_string(),
            region: "us-east-1".to_string(),
            root_path: "project/root".to_string(),
            compression: "zstd".to_string(),
            use_path_style: true,
            delete_other_versions: true,
        };

        save("dev", &profile).expect("save");

        let loaded = load("dev").expect("load");
        assert_eq!(loaded.bucket, "bucket-a");
        assert_eq!(loaded.compression, "zstd");

        let items = list().expect("list");
        assert_eq!(items, vec!["dev".to_string()]);

        delete("dev").expect("delete");
        assert!(list().expect("list after delete").is_empty());
    }

    #[test]
    fn defaults_when_optional_fields_missing() {
        let home = tempfile::tempdir().expect("tempdir");
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", home.path());
        struct Guard(Option<String>);
        impl Drop for Guard {
            fn drop(&mut self) {
                if let Some(home) = &self.0 {
                    std::env::set_var("HOME", home);
                } else {
                    std::env::remove_var("HOME");
                }
            }
        }
        let _guard = Guard(old_home);
        let dir = home
            .path()
            .join(".config/s3-lfs/profiles/legacy-compression");
        fs::create_dir_all(&dir).expect("mkdir");

        let content = r#"{
  "access_key_id": "AKIA",
  "secret_access_key": "secret",
  "bucket": "bucket",
  "endpoint": "https://s3.example.com",
  "region": "us-east-1",
  "root_path": "",
  "use_path_style": false
}
"#;

        fs::write(dir.join("credentials.json"), content).expect("write");
        let loaded = load("legacy-compression").expect("load");
        assert_eq!(loaded.compression, DEFAULT_COMPRESSION);
        assert!(loaded.delete_other_versions);
    }
}
