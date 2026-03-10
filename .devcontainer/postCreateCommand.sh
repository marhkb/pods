#!/bin/bash

rustup toolchain install nightly
rustup +nightly component add rustfmt
rustup default nightly

./build-aux/devcontainer/build-deps.sh
