#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

gem install copyright-header
brew install fd

copyright-header --add-path "$BASEDIR"/src/cwl-lib/src/lib.rs \
                --guess-extension \
                 --license ASL2 \
                 --copyright-holder 'Kitten Cat LLC' \
                 --copyright-software 'cwl-mount' \
                 --copyright-software-description "Mount AWS CloudWatch logs as a file system." \
                 --copyright-year "2021" \
                 --output-dir /tmp \
                 --dry-run
