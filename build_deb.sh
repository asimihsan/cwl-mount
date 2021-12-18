#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

(cd "${BASEDIR}"/src && cargo build --workspace --profile production)
cp "${BASEDIR}"/cwl-mount-post-install.sh "${BASEDIR}"/src/target/production/cwl-mount-post-install.sh
(cd "${BASEDIR}" && fpm \
    --force \
    --output-type deb \
    --depends "libfuse-dev >= 2.6.0" \
    --depends "libcap2-bin" \
    --package \
    pkg/cwl-mount-0.1.2-1-x86_64.deb)

rsync -av "$BASEDIR"/src/target/production/cwl-mount "$BASEDIR"/pkg/cwl-mount
(cd "$BASEDIR"/pkg && tar -czvf cwl-mount-0.1.2-linux-x64_64.tar.gz cwl-mount)
rm -f "$BASEDIR"/pkg/cwl-mount