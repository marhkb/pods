#!/bin/bash

set -e

if [[ -n "${MANIFEST_SOURCED:-}" ]]; then
    return 0
fi
readonly MANIFEST_SOURCED=1

source "$(dirname "$0")/paths.sh"

manifest=$(jq -r '[
  .runtime,
  ."runtime-version",
  .sdk,
  .command,
  ."build-options"."append-path",
  (."build-options"."build-args" | join ("|")),
  (."build-options".env | to_entries | map("--env=\(.key)=\(.value)") | join("|")),
  (.modules[] | select(.name == "pods") | ."config-opts" | join("|")),
  (."finish-args" | join("|"))
] | @tsv' "${paths[manifest]}")

IFS=$'\t' read -r \
  manifest_runtime \
  manifest_runtime_version \
  manifest_sdk \
  manifest_command \
  manifest_append_path \
  manifest_build_args_str \
  manifest_extra_envs_str \
  manifest_config_opts_str \
  manifest_finish_args_str \
  <<< "$manifest"

IFS='|' read -r -a manifest_build_args <<< "$manifest_build_args_str"
IFS='|' read -r -a manifest_extra_envs <<< "$manifest_extra_envs_str"
IFS='|' read -r -a manifest_config_opts <<< "$manifest_config_opts_str"
IFS='|' read -r -a manifest_finish_args <<< "$manifest_finish_args_str"
