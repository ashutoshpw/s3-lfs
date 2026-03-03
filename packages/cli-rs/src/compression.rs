use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use base64::Engine;
use flate2::Compression;

use crate::config::CompressionKind;

pub struct CompressedFile {
    pub temp: tempfile::NamedTempFile,
    pub size: i64,
    pub checksum_crc32c_b64: String,
}

pub fn compress_to_temp(source_path: &Path, kind: CompressionKind) -> Result<CompressedFile> {
    let mut source = File::open(source_path)?;
    let mut temp = tempfile::NamedTempFile::new()?;

    match kind {
        CompressionKind::None => {
            std::io::copy(&mut source, &mut temp)?;
            temp.flush()?;
        }
        CompressionKind::Gzip => {
            let mut encoder =
                flate2::write::GzEncoder::new(temp.as_file_mut(), Compression::best());
            std::io::copy(&mut source, &mut encoder)?;
            encoder.finish()?;
        }
        CompressionKind::Zstd => {
            let mut encoder = zstd::Encoder::new(temp.as_file_mut(), 22)?;
            encoder.include_checksum(true)?;
            std::io::copy(&mut source, &mut encoder)?;
            encoder.finish()?;
        }
    }

    let size = temp.as_file().metadata()?.len() as i64;
    let checksum_crc32c_b64 = checksum_file_crc32c_b64(temp.path())?;

    Ok(CompressedFile {
        temp,
        size,
        checksum_crc32c_b64,
    })
}

pub fn decompress_to_file(
    source_path: &Path,
    destination_path: &Path,
    kind: CompressionKind,
) -> Result<()> {
    let source = File::open(source_path)?;
    let mut destination = File::create(destination_path)?;

    match kind {
        CompressionKind::None => {
            let mut reader = source;
            std::io::copy(&mut reader, &mut destination)?;
        }
        CompressionKind::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(source);
            std::io::copy(&mut decoder, &mut destination)?;
        }
        CompressionKind::Zstd => {
            let mut decoder = zstd::Decoder::new(source)?;
            std::io::copy(&mut decoder, &mut destination)?;
        }
    }

    destination.flush()?;
    Ok(())
}

fn checksum_file_crc32c_b64(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut checksum: u32 = 0;
    let mut buf = [0u8; 256 * 1024];

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        checksum = crc32c::crc32c_append(checksum, &buf[..n]);
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(checksum.to_be_bytes());
    Ok(b64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn round_trip_gzip() {
        let dir = tempfile::tempdir().expect("tmp");
        let src = dir.path().join("in.bin");
        let out = dir.path().join("out.bin");
        fs::write(&src, b"aaaaabbbbbccccccddddeeeee").expect("write");

        let compressed = compress_to_temp(&src, CompressionKind::Gzip).expect("compress");
        decompress_to_file(compressed.temp.path(), &out, CompressionKind::Gzip)
            .expect("decompress");

        assert_eq!(fs::read(&src).expect("src"), fs::read(&out).expect("out"));
    }
}
