#!/bin/bash

for dir in ./.flatpak-builder/rofiles/*; do
    if [ -d "$dir" ]; then
        sudo umount -l "$dir"
    fi
done

rm -rf ./.flatpak ./.flatpak-builder ./_build
