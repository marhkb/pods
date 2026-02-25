#!/bin/bash

source "$(dirname "$0")/paths.sh"

flatpak-builder --run "${paths[repo_dir]}" "${paths[manifest]}" bash
