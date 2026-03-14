FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

RUN apt-get update && apt-get install --no-install-recommends -y \
        flatpak \
        flatpak-builder \
        elfutils \
        rustup \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

COPY <<EOF /usr/local/bin/cargo-fmt
#!/bin/bash
exec .root/bin/cargo fmt "\$@"
EOF

RUN flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
RUN flatpak -y install \
        org.gnome.Sdk//50 \
        org.gnome.Platform//50 \
        org.freedesktop.Sdk.Extension.rust-stable//25.08 \
        org.freedesktop.Sdk.Extension.llvm21//25.08

RUN mkdir -p /usr/lib/sdk/rust-stable/lib/rustlib/src/rust && \
    ln -s \
    /var/lib/flatpak/runtime/org.freedesktop.Sdk.Extension.rust-stable/x86_64/25.08/active/files/lib/rustlib/src/rust/library \
    /usr/lib/sdk/rust-stable/lib/rustlib/src/rust/library

COPY <<EOF /usr/local/bin/flatpak-wrapper
#!/bin/bash
exec flatpak-builder --run \\
    /workspaces/pods/.flatpak/repo \\
    build-aux/com.github.marhkb.Pods.Devel.json \\
    "\$(basename "\$0")" "\$@"
EOF
RUN chmod +x /usr/local/bin/flatpak-wrapper
RUN ln -s /usr/local/bin/flatpak-wrapper /usr/local/bin/rustc && \
    ln -s /usr/local/bin/flatpak-wrapper /usr/local/bin/cargo && \
    ln -s /usr/local/bin/flatpak-wrapper /usr/local/bin/rust-analyzer && \
    ln -s /usr/local/bin/flatpak-wrapper /usr/local/bin/blueprint-compiler
