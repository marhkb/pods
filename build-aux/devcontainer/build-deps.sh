#!/bin/bash

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

"$(dirname "$0")/init.sh"

set -e

flatpak-builder \
  --ccache \
  --force-clean \
  --disable-updates \
  --download-only \
  --state-dir="${paths[state_dir]}" \
  --stop-at="${manifest_command}" \
  "${paths[repo_dir]}" \
  "${paths[manifest]}"

flatpak-builder \
  --ccache \
  --force-clean \
  --disable-updates \
  --disable-download \
  --build-only \
  --keep-build-dirs \
  --state-dir="${paths[state_dir]}" \
  --stop-at="${manifest_command}" \
  "${paths[repo_dir]}" \
  "${paths[manifest]}"
