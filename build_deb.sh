#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

(cd "${BASEDIR}"/src && cargo build --workspace --release)
(cd "${BASEDIR}" && fpm \
    --force \
    --output-type deb \
    --depends "libfuse-dev >= 2.6.0" \
    --package \
    pkg/cwl-mount-0.1.1-1-x86_64.deb)
