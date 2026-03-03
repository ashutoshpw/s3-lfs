use std::io::{self, Write};

pub fn print_usage(writer: &mut dyn Write) -> io::Result<()> {
    writeln!(writer, "Usage:")?;
    writeln!(writer, "  s3-lfs [flags]")?;
    writeln!(writer, "  s3-lfs setup [--profile <slug>] [flags]")?;
    writeln!(writer, "  s3-lfs profile list")?;
    writeln!(writer, "  s3-lfs profile show --profile <slug>")?;
    writeln!(writer, "  s3-lfs profile delete --profile <slug>")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "Without a subcommand, s3-lfs runs as a Git LFS custom transfer agent."
    )?;
    writeln!(writer, "Setup is interactive:")?;
    writeln!(
        writer,
        "  - s3-lfs setup: choose existing profile to edit or add a new profile"
    )?;
    writeln!(
        writer,
        "  - s3-lfs setup --profile <slug>: edit/create a specific profile"
    )?;
    writeln!(writer)?;
    writeln!(writer, "Transfer-agent/setup flags:")?;
    writeln!(
        writer,
        "  --profile string                 Named profile slug from ~/.config/s3-lfs/profiles/<slug>/credentials.json"
    )?;
    writeln!(
        writer,
        "  --access_key_id string           S3 access key ID"
    )?;
    writeln!(
        writer,
        "  --secret_access_key string       S3 secret access key"
    )?;
    writeln!(writer, "  --bucket string                  S3 bucket")?;
    writeln!(writer, "  --endpoint string                S3 endpoint")?;
    writeln!(writer, "  --region string                  S3 region")?;
    writeln!(
        writer,
        "  --root_path string               Path inside bucket to store LFS objects"
    )?;
    writeln!(
        writer,
        "  --use_path_style[=true|false]    Use path-style S3 URLs (default: false)"
    )?;
    writeln!(
        writer,
        "  --delete_other_versions[=true|false] Delete alternate compression variants (default: true)"
    )?;
    writeln!(
        writer,
        "  --compression string             Compression: zstd, gzip, none (default: none)"
    )?;
    writeln!(writer)?;
    writeln!(writer, "Profile commands:")?;
    writeln!(writer, "  list:   list configured profiles")?;
    writeln!(writer, "  show:   print profile JSON")?;
    writeln!(writer, "  delete: delete a profile directory")?;
    Ok(())
}
