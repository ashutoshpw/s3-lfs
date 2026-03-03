#!/bin/sh
set -ex

if [ -d "./packages/cli/cmd/s3-lfs" ]; then
  GO_ENTRY="./packages/cli/cmd/s3-lfs"
elif [ -d "./cmd/s3-lfs" ]; then
  GO_ENTRY="./cmd/s3-lfs"
else
  echo "Could not find Go CLI entrypoint. Checked ./packages/cli/cmd/s3-lfs and ./cmd/s3-lfs"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon is not available. Start Docker and retry."
  exit 1
fi

CGO_ENABLED=0 GOOS=linux go build -o packages/cli/test/s3-lfs "$GO_ENTRY"
docker build -f packages/cli/test/Dockerfile -t s3-lfs-test .
docker run --rm -t s3-lfs-test
