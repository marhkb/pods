#!/bin/bash

set -e

source "$(dirname "$0")/paths.sh"
source "$(dirname "$0")/manifest.sh"

flatpak build \
  "${manifest_finish_args[@]}" \
  --with-appdir \
  --allow=devel \
  --bind-mount="${XDG_RUNTIME_DIR}"/doc="${XDG_RUNTIME_DIR}"/doc/by-app/"${id}" \
  --bind-mount=/run/flatpak/at-spi-bus="${XDG_RUNTIME_DIR}"/at-spi/bus \
  --bind-mount=/run/host/fonts=/usr/share/fonts \
  --talk-name=org.freedesktop.portal.* \
  --talk-name=org.a11y.Bus \
  --env=AT_SPI_BUS_ADDRESS=unix:path=/run/flatpak/at-spi-bus \
  --env=DESKTOP_SESSION="${DESKTOP_SESSION}" \
  --env=LANG="${LANG}" \
  --env=WAYLAND_DISPLAY="${WAYLAND_DISPLAY}" \
  --env=XDG_CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP}" \
  --env=XDG_SESSION_DESKTOP="${XDG_SESSION_DESKTOP}" \
  --env=XDG_SESSION_TYPE="${XDG_SESSION_TYPE}" \
  "${paths[repo_dir]}" \
  "${manifest_command}"
