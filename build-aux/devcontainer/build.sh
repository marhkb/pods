#!/bin/bash

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

"$(dirname "$0")/build-deps.sh"

set -e

args=(
  "${manifest_build_args[@]}"
  "--filesystem="${paths[project_dir]}""
  "--filesystem="${paths[repo_dir]}""
  "--filesystem="${paths[system_build_dir]}""
  "--env=PATH=/usr/lib64/ccache:/usr/local/sbin:/usr/local/bin:/usr/bin:/app/bin"${manifest_append_path:+:$manifest_append_path}""
  "--env=LD_LIBRARY_PATH=/app/lib"
  "--env=PKG_CONFIG_PATH=/app/lib/pkgconfig:/app/share/pkgconfig:/usr/lib/pkgconfig:/usr/share/pkgconfig"
  "${manifest_extra_envs[@]}"
)

flatpak build \
  "${args[@]}" \
  "${paths[repo_dir]}" \
  meson setup \
  --prefix=/app \
  ${paths[system_build_dir]} \
  "${manifest_config_opts[@]}"

flatpak build \
  "${args[@]}" \
  "${paths[repo_dir]}" \
  ninja \
  -C ${paths[system_build_dir]}

flatpak build \
  "${args[@]}" \
  "${paths[repo_dir]}" \
  meson install \
  -C ${paths[system_build_dir]}
