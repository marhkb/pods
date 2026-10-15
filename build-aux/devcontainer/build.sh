#!/bin/bash

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

"$(dirname "$0")/build-deps.sh"

set -e

run() {
  "$(dirname "$0")/open-build-terminal.sh" "$@"
}

# Reconfigure an existing build directory, so that added or removed Blueprint files are picked up.
setup_opts=()
if [[ -d "${paths[system_build_dir]}/meson-private" ]]; then
  setup_opts+=(--reconfigure)
fi

run meson setup "${setup_opts[@]}" --prefix=/app "${paths[system_build_dir]}" "${manifest_config_opts[@]}"
run ninja -C "${paths[system_build_dir]}"
run meson install -C "${paths[system_build_dir]}"
