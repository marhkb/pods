#!/bin/bash
set -e

PREFIX="$(pwd)/.flatpak/app"

export CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="-C link-arg=-fuse-ld=/usr/lib/sdk/rust-stable/bin/mold"

if [ ! -d "_build" ]; then
    meson setup _build --prefix="$PREFIX" -Dprofile=development
fi

ninja -C _build install

export XDG_DATA_DIRS="$PREFIX/share:$XDG_DATA_DIRS"
export PATH="$PREFIX/bin:$PATH"
export GSETTINGS_SCHEMA_DIR="$PREFIX/share/glib-2.0/schemas"

pods
