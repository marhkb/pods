#!/bin/bash

source "$(dirname "$0")/paths.sh"

"$(dirname "$0")/open-build-terminal.sh" ninja -C "${paths[system_build_dir]}" pods-update-po
