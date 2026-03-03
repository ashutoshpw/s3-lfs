use anyhow::{bail, Result};

pub const DEFAULT_COMPRESSION: &str = "none";
pub const COMPRESSION_PREFERENCE: [CompressionKind; 3] = [
    CompressionKind::Zstd,
    CompressionKind::Gzip,
    CompressionKind::None,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionKind {
    Zstd,
    Gzip,
    None,
}

impl CompressionKind {
    pub fn name(self) -> &'static str {
        match self {
            CompressionKind::Zstd => "zstd",
            CompressionKind::Gzip => "gzip",
            CompressionKind::None => "none",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            CompressionKind::Zstd => ".zstd",
            CompressionKind::Gzip => ".gz",
            CompressionKind::None => "",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "zstd" => Some(Self::Zstd),
            "gzip" => Some(Self::Gzip),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub profile: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
    pub root_path: String,
    pub use_path_style: bool,
    pub delete_other_versions: bool,
    pub compression: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            profile: String::new(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            bucket: String::new(),
            endpoint: String::new(),
            region: String::new(),
            root_path: String::new(),
            use_path_style: false,
            delete_other_versions: true,
            compression: DEFAULT_COMPRESSION.to_string(),
        }
    }
}

impl RuntimeConfig {
    pub fn with_profile(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
            ..Self::default()
        }
    }

    pub fn compression_kind(&self) -> Result<CompressionKind> {
        CompressionKind::parse(&self.compression)
            .ok_or_else(|| anyhow::anyhow!("invalid compression set: {}", self.compression))
    }

    pub fn validate(&self) -> Result<()> {
        if self.bucket.is_empty() {
            bail!("no bucket set");
        }
        if self.endpoint.is_empty() {
            bail!("no endpoint set");
        }
        if self.region.is_empty() {
            bail!("no region set; configure --region, AWS_REGION, or profile region");
        }
        if (self.access_key_id.is_empty()) != (self.secret_access_key.is_empty()) {
            bail!("access key and secret key should either both be set or both be empty");
        }
        if CompressionKind::parse(&self.compression).is_none() {
            bail!("invalid compression set");
        }
        Ok(())
    }
}
