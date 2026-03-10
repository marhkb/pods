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
