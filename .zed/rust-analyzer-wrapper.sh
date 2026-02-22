#!/bin/bash

exec flatpak-builder --run \
    $(pwd)/.flatpak/repo \
    build-aux/com.github.marhkb.Pods.Devel.json \
    rust-analyzer "$@"
