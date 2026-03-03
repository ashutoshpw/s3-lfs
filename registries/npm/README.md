# s3-lfs (npm)

This package installs the `s3-lfs` executable and exposes it on your PATH.
It also provides `s3lfs` as an alias.

## Install

```bash
npm install -g s3-lfs
```

## What happens on install

- `postinstall` downloads the latest release asset from GitHub.
- Default asset flavor: Rust (`rs`)
- Asset naming convention:
  - Linux: `s3-lfs-rs-linux-amd64`, `s3-lfs-rs-linux-arm64`
  - macOS: `s3-lfs-rs-macos-amd64`, `s3-lfs-rs-macos-arm64`
  - Windows: `s3-lfs-rs-windows-amd64.exe`
- Binary is stored under `vendor/` and executed via `bin/s3-lfs.js`.
- Current package supports `amd64` and `arm64` where release assets are available.

## Overrides

- `LFS_S3_NPM_REPO` (default: `ashutoshpw/s3-lfs`)
- `LFS_S3_NPM_TAG` (example: `v1.2.3`)
- `LFS_S3_NPM_FLAVOR` (`rs` default, or `go`)
- `LFS_S3_NPM_SKIP_DOWNLOAD=1` (skip installer)

## Verify

```bash
s3-lfs --help
s3lfs --help
```

## Issues

If you find a bug or have a feature request, please open an issue at [github.com/ashutoshpw/s3-lfs/issues](https://github.com/ashutoshpw/s3-lfs/issues).
