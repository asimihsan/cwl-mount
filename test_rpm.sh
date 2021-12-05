#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

yum -y localinstall "${BASEDIR}"/pkg/cwl-mount-0.1.0-1-x86_64.rpm
/usr/bin/cwl-mount /tmp/foo --log-group-name babynames-preprod-log-group-syslog
