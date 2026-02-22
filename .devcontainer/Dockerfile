FROM mcr.microsoft.com/devcontainers/rust

RUN apt-get update && apt-get install -y flatpak flatpak-builder

RUN flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
RUN flatpak -y install org.gnome.Sdk//49 \
    org.gnome.Platform//49 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08 \
    org.freedesktop.Sdk.Extension.llvm21//25.08

RUN rustup toolchain install nightly && rustup +nightly component add rustfmt

RUN mkdir -p /usr/lib/sdk/rust-stable/lib/rustlib/src/rust && \
    ln -s \
    /var/lib/flatpak/runtime/org.freedesktop.Sdk.Extension.rust-stable/x86_64/25.08/active/files/lib/rustlib/src/rust/library \
    /usr/lib/sdk/rust-stable/lib/rustlib/src/rust/library
