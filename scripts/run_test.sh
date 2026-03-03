#!/bin/sh
set -ex

CGO_ENABLED=0 GOOS=linux go build -o packages/cli/test/s3-lfs ./packages/cli/cmd/s3-lfs
docker build -f packages/cli/test/Dockerfile -t s3-lfs-test .
docker run --rm -t s3-lfs-test
