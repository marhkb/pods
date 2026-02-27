#!/bin/bash

set -e

if [[ -n "${PATHS_SOURCED:-}" ]]; then
    return 0
fi
readonly PATHS_SOURCED=1

id=com.github.marhkb.Pods.Devel

declare -A paths
paths[project_dir]=$(realpath "$(dirname "$0")/../..")
paths[repo_dir]="${paths[project_dir]}/.flatpak/repo"
paths[state_dir]="${paths[project_dir]}/.flatpak/flatpak-builder"
paths[system_build_dir]="${paths[project_dir]}/_build"
paths[manifest]="${paths[project_dir]}/build-aux/${id}.json"
