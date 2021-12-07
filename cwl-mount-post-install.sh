#!/usr/bin/env bash

set -euxo pipefail

/sbin/setcap cap_sys_admin+ep /usr/bin/cwl-mount
