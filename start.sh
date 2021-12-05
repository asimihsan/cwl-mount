#!/usr/bin/env bash

uuid=$(uuid)
tmux new -d -s "$uuid"
tmux splitw -v -t "${uuid}:0.0"
tmux send-keys -t "${uuid}.0" "cwl-mount --help" ENTER
tmux a -t "$uuid"
