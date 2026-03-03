# Repository Guidelines

## Project Structure & Module Organization
- `packages/cli/cmd/s3-lfs/main.go`: binary entrypoint.
- `packages/cli/`: command parsing, setup/profile flows, and runtime config resolution.
- `packages/cli/api/`: line-oriented JSON protocol types and response helpers.
- `packages/cli/service/`: request handling loop and transfer orchestration.
- `packages/cli/s3adapter/`: S3 config, upload/download, signing, and storage behavior.
- `packages/cli/compression/`: compression implementations (`zstd`, `gzip`, `none`).
- `packages/cli/test/`: integration Docker assets.
- `scripts/`: runnable shell helpers (`scripts/run_test.sh`, `scripts/integration/run.sh`, `scripts/update_deps.sh`).
- `registries/npm/`: JavaScript npm wrapper package for distributing the `s3-lfs` executable.
- `makefiles/` and `packaging/windows/`: build/packaging assets, including MSI template.
## Build, Test, and Development Commands
- `go build -v ./...`: compile all packages locally.
- `go test -v ./...`: run unit tests (matches CI in `.github/workflows/build.yml`).
- `./scripts/run_test.sh`: build Linux binary and run Docker-based integration test.
- `scripts/integration/run.sh $(pwd)/.envrc`: run integration tests against your own S3 provider credentials.
- `go mod tidy`: normalize module dependencies (also done in release workflow).

## Coding Style & Naming Conventions
- Follow standard Go style and always run `gofmt` before committing.
- Use lowercase package names; exported identifiers in `PascalCase`; unexported in `camelCase`.
- Keep flag/config wiring and CLI behavior in `packages/cli`, protocol message definitions in `packages/cli/api`, and S3-specific logic in `packages/cli/s3adapter`.
- Prefer small, focused functions and explicit error returns.

## Testing Guidelines
- Place unit tests in `*_test.go` files next to the code they cover.
- Name tests with `TestXxx` and use table-driven tests for protocol/config edge cases.
- Run `go test -v ./...` for every change; run `./scripts/run_test.sh` when touching transfer, compression, or S3 behavior.
- Integration tests require Docker; `scripts/integration/run.sh` can start local MinIO if no `.envrc` is provided.

## Commit & Pull Request Guidelines
- Use concise, imperative commit messages; prefixes like `feat:`, `tests:`, `build(deps):`, `README:`, and `.github:` are common in history.
- Keep commits scoped to one logical change.
- PRs should target `main`, include a clear summary, linked issue (if any), and exact test commands run.
- Call out config or credential-handling changes explicitly.

## Security & Configuration Tips
- Never commit real access keys, secrets, or bucket-specific credentials.
- Keep local credentials in untracked env files (for example `.envrc`) and sanitize logs before sharing.
