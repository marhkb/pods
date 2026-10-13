#!/bin/bash

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

set -e

if [[ ! -d "${paths[system_build_dir]}" ]]; then
  echo "Build directory not found. Run build.sh first." >&2
  exit 1
fi

args=(
  "${manifest_build_args[@]}"
  "--filesystem="${paths[project_dir]}""
  "--filesystem="${paths[repo_dir]}""
  "--filesystem="${paths[system_build_dir]}""
  "--env=PATH=/usr/lib64/ccache:/usr/local/sbin:/usr/local/bin:/usr/bin:/app/bin"${manifest_append_path:+:$manifest_append_path}""
  "${manifest_extra_envs[@]}"
)

flatpak build \
  "${args[@]}" \
  "${paths[repo_dir]}" \
  ninja \
  -C ${paths[system_build_dir]} \
  pods-update-po
