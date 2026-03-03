use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::RuntimeConfig;

#[derive(Clone, Debug, Default)]
pub struct RepoConfig {
    pub has_root_path: bool,
    pub root_path: String,
    pub has_compression: bool,
    pub compression: String,
}

pub fn find_repo_root(start: &Path) -> io::Result<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(".git").exists() {
            return Ok(dir);
        }

        if !dir.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "repo root not found",
            ));
        }
    }
}

pub fn resolve_repo_config(start: &Path) -> io::Result<RepoConfig> {
    let repo_root = find_repo_root(start)?;
    parse_lfsconfig(&repo_root.join(".lfsconfig"))
}

pub fn parse_lfsconfig(path: &Path) -> io::Result<RepoConfig> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(RepoConfig::default()),
        Err(err) => return Err(err),
    };

    let mut cfg = RepoConfig::default();
    let mut section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }

        if section != "s3-lfs" {
            continue;
        }

        let Some((k, v)) = parse_kv(line) else {
            continue;
        };

        let key = k.trim().to_ascii_lowercase().replace('-', "_");
        let value = trim_config_value(v.trim()).to_string();

        match key.as_str() {
            "root_path" => {
                cfg.root_path = value;
                cfg.has_root_path = true;
            }
            "compression" => {
                cfg.compression = value.to_ascii_lowercase();
                cfg.has_compression = true;
            }
            _ => {}
        }
    }

    Ok(cfg)
}

pub fn apply_repo_overrides(
    dst: &mut RuntimeConfig,
    repo: &RepoConfig,
    explicit: &HashSet<String>,
) {
    if repo.has_root_path && !explicit.contains("root_path") {
        dst.root_path = repo.root_path.clone();
    }
    if repo.has_compression && !explicit.contains("compression") {
        dst.compression = repo.compression.clone();
    }
}

fn parse_kv(line: &str) -> Option<(String, String)> {
    if let Some(idx) = line.find('=') {
        let key = line[..idx].trim();
        if key.is_empty() {
            return None;
        }
        return Some((key.to_string(), line[idx + 1..].trim().to_string()));
    }

    let mut fields = line.split_whitespace();
    let key = fields.next()?.to_string();
    let value = fields.collect::<Vec<_>>().join(" ");
    if value.is_empty() {
        return None;
    }
    Some((key, value))
}

fn trim_config_value(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        return &value[1..value.len() - 1];
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuntimeConfig;

    #[test]
    fn parse_section() {
        let dir = tempfile::tempdir().expect("tmp");
        let file = dir.path().join(".lfsconfig");
        fs::write(
            &file,
            "[s3-lfs]\nroot-path = team/repo\ncompression = gzip\n",
        )
        .expect("write");

        let cfg = parse_lfsconfig(&file).expect("parse");
        assert!(cfg.has_root_path);
        assert_eq!(cfg.root_path, "team/repo");
        assert!(cfg.has_compression);
        assert_eq!(cfg.compression, "gzip");
    }

    #[test]
    fn apply_repo_overrides_respects_explicit_flags() {
        let repo = RepoConfig {
            has_root_path: true,
            root_path: "repo/root".to_string(),
            has_compression: true,
            compression: "gzip".to_string(),
        };

        let mut cfg = RuntimeConfig {
            root_path: "profile/root".to_string(),
            compression: "none".to_string(),
            ..RuntimeConfig::default()
        };
        apply_repo_overrides(&mut cfg, &repo, &HashSet::new());
        assert_eq!(cfg.root_path, "repo/root");
        assert_eq!(cfg.compression, "gzip");

        let explicit = HashSet::from(["root_path".to_string(), "compression".to_string()]);
        cfg.root_path = "profile/root".to_string();
        cfg.compression = "none".to_string();
        apply_repo_overrides(&mut cfg, &repo, &explicit);
        assert_eq!(cfg.root_path, "profile/root");
        assert_eq!(cfg.compression, "none");
    }
}
