#!/bin/sh

set -eu

meson setup --prefix=/usr --buildtype=release _build
meson install -C _build

ARCH=$(uname -m)
VERSION=$(meson introspect --projectinfo _build | jq -r .version)
export ARCH VERSION
export OUTPATH=./dist
export ADD_HOOKS="self-updater.hook"
export UPINFO="gh-releases-zsync|marhkb|pods|latest|*$ARCH.AppImage.zsync"
export ICON=/usr/share/icons/hicolor/scalable/apps/com.github.marhkb.Pods.svg
export DESKTOP=/usr/share/applications/com.github.marhkb.Pods.desktop
export STARTUPWMCLASS=com.github.marhkb.Pods
export GTK_CLASS_FIX=1
export USE_HOST_DRIVERS_EXPERIMENTAL=1

quick-sharun /usr/bin/pods

quick-sharun --make-appimage

quick-sharun --test ./dist/*.AppImage
