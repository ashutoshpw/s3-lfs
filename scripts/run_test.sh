#!/bin/sh
set -ex

GO_ENTRY=""
for candidate in \
  "./packages/cli/cmd/s3-lfs" \
  "./packages/cli/cmd/lfs-s3" \
  "./cmd/s3-lfs" \
  "./cmd/lfs-s3"
do
  if [ -f "$candidate/main.go" ]; then
    GO_ENTRY="$candidate"
    break
  fi
done

if [ -z "$GO_ENTRY" ]; then
  echo "Could not find Go CLI entrypoint."
  echo "Checked: ./packages/cli/cmd/s3-lfs ./packages/cli/cmd/lfs-s3 ./cmd/s3-lfs ./cmd/lfs-s3"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon is not available. Start Docker and retry."
  exit 1
fi

echo "Using Go CLI entrypoint: $GO_ENTRY"
CGO_ENABLED=0 GOOS=linux go build -o packages/cli/test/s3-lfs "$GO_ENTRY"
docker build -f packages/cli/test/Dockerfile -t s3-lfs-test .
docker run --rm -t s3-lfs-test
