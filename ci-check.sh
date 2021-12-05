#!/usr/bin/env bash

set -euxo pipefail

# cargo install cargo-pants
cargo pants

cargo deny check