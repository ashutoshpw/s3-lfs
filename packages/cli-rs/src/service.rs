use std::io::{self, BufRead, Write};

use anyhow::{anyhow, bail, Result};

use crate::config::RuntimeConfig;
use crate::protocol::{send_init, send_progress, send_transfer, Request};
use crate::s3_adapter::S3Adapter;

pub fn serve(
    stdin: &mut dyn io::Read,
    stdout: &mut dyn Write,
    _stderr: &mut dyn Write,
    config: &RuntimeConfig,
) -> Result<()> {
    config.validate()?;

    let adapter = S3Adapter::new(config)?;
    let reader = io::BufReader::new(stdin);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: Request =
            serde_json::from_str(&line).map_err(|e| anyhow!("error reading input: {}", e))?;

        match req.event.as_str() {
            "init" => {
                send_init(0, None, stdout)?;
            }
            "terminate" => {
                // Graceful no-op termination.
            }
            "download" => {
                let local_path = local_path(&req.oid)?;
                let mut bytes_so_far = 0i64;
                let mut callback = |transferred: i64| {
                    bytes_so_far += transferred;
                    let _ = send_progress(&req.oid, bytes_so_far, transferred, stdout);
                };

                match adapter.download(&req.oid, &local_path, &mut callback) {
                    Ok(()) => {
                        send_transfer(&req.oid, 0, None, Some(&local_path), stdout)?;
                    }
                    Err(err) => {
                        send_transfer(&req.oid, 1, Some(&err), Some(&local_path), stdout)?;
                    }
                }
            }
            "upload" => {
                let mut bytes_so_far = 0i64;
                let mut callback = |transferred: i64| {
                    bytes_so_far += transferred;
                    let _ = send_progress(&req.oid, bytes_so_far, transferred, stdout);
                };

                match adapter.upload(&req.oid, &req.path, &mut callback) {
                    Ok(()) => {
                        send_transfer(&req.oid, 0, None, None, stdout)?;
                    }
                    Err(err) => {
                        send_transfer(&req.oid, 1, Some(&err), None, stdout)?;
                    }
                }
            }
            _ => {
                // Ignore unknown events to match Go behavior.
            }
        }
    }

    Ok(())
}

fn local_path(oid: &str) -> Result<String> {
    if !is_valid_oid(oid) {
        bail!("Invalid lfs object ID {}", oid);
    }

    Ok(format!(
        ".git/lfs/objects/{}/{}/{}",
        &oid[..2],
        &oid[2..4],
        oid
    ))
}

fn is_valid_oid(oid: &str) -> bool {
    oid.len() == 64
        && oid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_path_requires_valid_oid() {
        let bad = local_path("xyz");
        assert!(bad.is_err());

        let good = local_path("188dd802cc9e1b686b9889adc523300ab0b2a8a461ae8eb10e6578cb244f90ad")
            .expect("path");
        assert!(good.contains(".git/lfs/objects/18/8d/"));
    }
}
