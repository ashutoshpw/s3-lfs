FROM alpine:3.21.0

RUN apk add --no-cache curl git git-lfs minio minio-client

RUN git config --global user.email "s3-lfs-test@none.com"
RUN git config --global user.name "s3-lfs test"
RUN git config --global init.defaultBranch main

RUN mkdir /work
WORKDIR /work
RUN mkdir -p /work/minio_data
RUN chown -R minio:minio /work/minio_data
COPY packages/cli/test/s3-lfs /work/s3-lfs
COPY scripts/integration/run.sh /work/run.sh
CMD /work/run.sh
