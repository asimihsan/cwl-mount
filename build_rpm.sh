#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

(cd "${BASEDIR}"/src && cargo build --workspace --release)
cp "${BASEDIR}"/cwl-mount-post-install.sh "${BASEDIR}"/src/target/release/cwl-mount-post-install.sh
(cd "${BASEDIR}" && fpm \
    --force \
    --output-type rpm \
    --depends "fuse-devel >= 2.6.0" \
    --depends "fuse-libs >= 2.6.0" \
    --depends "libcap" \
    --package \
    pkg/cwl-mount-0.1.1-1-x86_64.rpm)
