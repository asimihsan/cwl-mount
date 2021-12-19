#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

cargo install cargo-pants
cargo install cargo-fuzz

(cd "$BASEDIR"/src && cargo pants || true)
(cd "$BASEDIR"/src && cargo deny check || true)
(cd "$BASEDIR"/src/format-cwl-log-event && ./fuzz.sh)
