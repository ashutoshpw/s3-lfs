# s3-lfs

[![Go Reference](https://pkg.go.dev/badge/github.com/ashutoshpw/s3-lfs.svg)](https://pkg.go.dev/github.com/ashutoshpw/s3-lfs)
![Build](https://github.com/ashutoshpw/s3-lfs/actions/workflows/build.yml/badge.svg)
![Test](https://github.com/ashutoshpw/s3-lfs/actions/workflows/test.yml/badge.svg)

A [Custom Transfer Agent](https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md) for [Git LFS](https://git-lfs.github.com/) that stores LFS binary files directly in an [S3 bucket](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html). No server required.

## Features

- Works with any S3-compatible storage (AWS S3, MinIO, GCS, Cloudflare R2, etc.)
- Named local profiles for easy multi-project credential management
- Optional compression (`zstd`, `gzip`, or `none`)
- Skips redundant re-uploads via S3 checksumming
- GCS compatibility workaround included

## Installation

**Using Go:**

```sh
go install github.com/ashutoshpw/s3-lfs/packages/cli/cmd/s3-lfs@latest
```

**Using npm:**

```sh
npm install -g s3-lfs
```

**Or download a binary** from the [releases page](https://github.com/ashutoshpw/s3-lfs/releases).

## Quick Start

### 1. Create a profile

```sh
s3-lfs setup
```

This launches interactive setup:
- if profiles exist, choose one to edit or add a new profile
- if no profiles exist, create a new profile

You can also target a profile directly and pass values as flags:

```sh
s3-lfs setup --profile my-profile \
  --access_key_id=AKIAEXAMPLE \
  --secret_access_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY \
  --bucket=my-lfs-bucket \
  --endpoint=https://s3.us-east-1.amazonaws.com \
  --region=us-east-1
```

### 2. Configure your git repo

```sh
git lfs install --local
git config --local --add lfs.customtransfer.s3-lfs.path s3-lfs
git config --local --add lfs.standalonetransferagent s3-lfs
git config --local --add lfs.customtransfer.s3-lfs.args '--profile=my-profile'
```

### 3. Use git as normal

```sh
git lfs track "*.bin"
git add .gitattributes large-file.bin
git commit -m "Add large file"
git push origin main
```

LFS files are now uploaded directly to your S3 bucket.

### Cloning a repo that uses s3-lfs

```sh
GIT_LFS_SKIP_SMUDGE=1 git clone <url>
cd <repo>

# Configure s3-lfs (same as step 2 above)
git config --add lfs.customtransfer.s3-lfs.path s3-lfs
git config --add lfs.standalonetransferagent s3-lfs
git config --add lfs.customtransfer.s3-lfs.args '--profile=my-profile'

git lfs pull
```

## Configuration

### Profile management

```sh
s3-lfs profile list
s3-lfs profile show --profile my-profile
s3-lfs profile delete --profile my-profile
```

Profiles are stored in `~/.config/s3-lfs/profiles/<profile-slug>/credentials.json`.

### Per-repo overrides

You can override `root_path` and `compression` per-repo via CLI args:

```sh
git config --add lfs.customtransfer.s3-lfs.args '--profile=my-profile --root_path=my/repo/prefix'
```

Or via `.lfsconfig`:

```ini
[s3-lfs]
	root-path = my/repo/prefix
	compression = gzip
```

### Precedence

- `root_path` and `compression`: profile value < `.lfsconfig` < explicit CLI flag
- All other settings: profile value < environment variable < explicit CLI flag

### Command-line flags

| Name                      | Description                                                                                      | Default | Required |
|---------------------------|--------------------------------------------------------------------------------------------------|---------|----------|
| `--profile`               | Named profile from `~/.config/s3-lfs/profiles/`                                                 |         | No       |
| `--access_key_id`         | S3 Access key ID                                                                                 |         | Yes      |
| `--secret_access_key`     | S3 Secret access key                                                                             |         | Yes      |
| `--bucket`                | S3 Bucket name                                                                                   |         | Yes      |
| `--endpoint`              | S3 Endpoint                                                                                      |         | Yes      |
| `--region`                | S3 Region                                                                                        |         | Yes      |
| `--root_path`             | Path within the bucket for LFS files                                                             |         | No       |
| `--compression`           | Compression method: `zstd`, `gzip`, or `none`                                                    | `none`  | No       |
| `--use_path_style`        | Use S3 SDK path-style option                                                                     | `false` | No       |
| `--delete_other_versions` | Delete other versions of a file (e.g. with different compression) after upload                   | `true`  | No       |

> [!IMPORTANT]
> When using `--profile`, the required flags above (`access_key_id`, etc.) are read from the profile and don't need to be passed explicitly.

### Alternative configuration (environment variables)

Instead of profiles or CLI flags, you can use environment variables:

| Variable              | Description                          |
|-----------------------|--------------------------------------|
| `S3_BUCKET`           | Bucket for LFS storage (required)    |
| `AWS_REGION`          | S3 region                            |
| `AWS_S3_ENDPOINT`     | S3 endpoint                          |
| `AWS_ACCESS_KEY_ID`   | Access key ID                        |
| `AWS_SECRET_ACCESS_KEY` | Secret access key                  |
| `AWS_CONFIG_FILE`     | Path to AWS config file              |
| `AWS_PROFILE`         | AWS profile name                     |

See the [AWS shared credentials docs](https://docs.aws.amazon.com/sdkref/latest/guide/file-format.html) for config file format.

> [!NOTE]
> If you set an environment variable, don't also set the same value as a CLI flag in your git LFS args.

## Testing

```sh
# Unit tests
go test -v ./...

# Integration test (Linux, requires Docker)
./scripts/run_test.sh

# Integration test with your own S3 credentials
scripts/integration/run.sh $(pwd)/.envrc
```

## Contributing

Pull requests are welcome. Please open an issue at [github.com/ashutoshpw/s3-lfs/issues](https://github.com/ashutoshpw/s3-lfs/issues) for bugs or feature requests.

## Compatibility

Tested with Git LFS >= 3.3.0.
