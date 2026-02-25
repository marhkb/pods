#!/bin/bash

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

set +e

built_init_msg=$(flatpak build-init \
    "${paths[repo_dir]}" \
    "${id}" \
    "${manifest_sdk}" \
    "${manifest_runtime}" \
    "${manifest_runtime_version}" \
    2>&1)

built_init_status=$?
if [[ "$built_init_status" -ne 0 ]]; then
    if [[ "${built_init_msg}" == *"already initialized"* ]]; then
        echo "Build directory "${paths[repo_dir]}" already initialized"
    else
        echo "${built_init_msg}"
        exit 1
    fi
fi

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
