#!/usr/bin/env bash

set -euxo pipefail

BASEDIR=$(realpath "$(dirname "$0")")

cat << 'EOF' > /tmp/format-cwl-log-event-dict.txt
# Dictionary file for format-cwl-log-event::LogFormatter::new()
"$"
"$$"
"{"
"}"
"log_group_name"
"event_id"
"ingestion_time"
"log_stream_name"
"message"
"timestamp"
"$log_group_name"
"$event_id"
"$ingestion_time"
"$log_stream_name"
"$message"
"$timestamp"
"${log_group_name}"
"${event_id}"
"${ingestion_time}"
"${log_stream_name}"
"${message}"
"${timestamp}"
"[$log_group_name] [$log_stream_name] $timestamp - $message"
EOF

(cd "$BASEDIR" && \
    cargo build --release && \
    cargo +nightly fuzz run fuzz_target_1 -- \
        -dict=/tmp/format-cwl-log-event-dict.txt \
        -max_total_time=60 \
        -max_len=1048576)
