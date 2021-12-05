#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

docker build . --file Dockerfile.amazonlinux2 --tag cwl-mount-al2:latest
docker run \
    --privileged \
    --interactive \
    --tty \
    --volume "$BASEDIR:/workspace" \
    --workdir /workspace \
    cwl-mount-al2:latest ./build_rpm.sh

docker build . --file Dockerfile.debian --tag cwl-mount-debian:latest
docker run \
    --privileged \
    --interactive \
    --tty \
    --volume "$BASEDIR:/workspace" \
    --workdir /workspace \
    cwl-mount-debian:latest ./build_deb.sh

(cd "$BASEDIR" && docker build . --file Dockerfile.runnable --tag cwl-mount:latest)
REGISTRY=b5u6b4p0
aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws/"$REGISTRY"
docker tag cwl-mount:latest public.ecr.aws/"$REGISTRY"/cwl-mount:latest
docker push public.ecr.aws/"$REGISTRY"/cwl-mount:latest

"$BASEDIR"/build_mac.sh