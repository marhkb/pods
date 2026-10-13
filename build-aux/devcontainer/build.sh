#!/bin/bash

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

"$(dirname "$0")/build-deps.sh"

set -e

run() {
  "$(dirname "$0")/open-build-terminal.sh" "$@"
}

run meson setup --prefix=/app "${paths[system_build_dir]}" "${manifest_config_opts[@]}"
run ninja -C "${paths[system_build_dir]}"
run meson install -C "${paths[system_build_dir]}"
