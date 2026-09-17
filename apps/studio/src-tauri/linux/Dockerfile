# قاعدة البناء لِلينكس — the old-baseline Linux build image.
#
# WHY THIS EXISTS.
#
# An AppImage bundles WebKitGTK and its 180-odd transitive libraries, but it
# never bundles glibc or libstdc++: those come from the host at load time. The
# floor is therefore set by whatever machine ran the build, and a build on a
# rolling distribution stamps that distribution's glibc onto the artifact. The
# 2026-09-04 build was made on glibc 2.42 and refuses to start anywhere older —
# including on every SteamOS release ever shipped (3.5 → 2.37, 3.6 → 2.39,
# 3.7 → 2.40, 3.8.1x → 2.41, read from Valve's own package mirror). The build
# host is the defect, not the code.
#
# Ubuntu 22.04 is the chosen baseline because it is the newest base whose glibc
# (2.35) sits below every SteamOS release, and because it is the oldest base
# that still carries a maintained `libwebkit2gtk-4.1` — Tauri v2 needs the 4.1
# (libsoup3) API, and 20.04 has only 4.0. That pair is what fixes the floor at
# GLIBC_2.35 / GLIBCXX_3.4.30, and it is the product's stated Linux floor.
#
# Anything that raises the base raises the floor. Do not "just bump" this tag.
FROM ubuntu:22.04

# Never let a package's post-install script open a prompt: nothing is attached.
ENV DEBIAN_FRONTEND=noninteractive

# Split into what the webview needs, what the build needs, and what the AppImage
# tooling needs, so a future removal can tell which is which.
RUN apt-get update && apt-get install --no-install-recommends -y \
      # the webview: Tauri v2 links webkit2gtk-4.1, which pulls libsoup3
      libwebkit2gtk-4.1-dev \
      libjavascriptcoregtk-4.1-dev \
      libsoup-3.0-dev \
      libgtk-3-dev \
      librsvg2-dev \
      # the tray/menu stack Tauri probes for at build time
      libayatana-appindicator3-dev \
      # keyring: the secret-service backend talks D-Bus
      libdbus-1-dev \
      # git2 is built with vendored libgit2 + vendored openssl; openssl's own
      # Configure script is Perl, and it will not build without make and a cc
      build-essential \
      perl \
      pkg-config \
      # rustup, the Node tarball, and linuxdeploy all arrive over the network
      curl \
      ca-certificates \
      xz-utils \
      file \
      git \
      # linuxdeploy rewrites RPATHs; without patchelf the AppDir is not portable
      patchelf \
      # desktop-file-validate, used by the bundler and by the verify script
      desktop-file-utils \
      # xdg-open, which the AppImage bundler copies out of /usr/bin into the
      # AppDir so the image can open a link on a host that has none of its own;
      # without it bundling stops at "xdg-open binary not found" and only the
      # .deb comes out. It cannot raise the floor this image exists to hold:
      # xdg-utils is Architecture: all and every program in it is a POSIX shell
      # script, so the AppDir gains no ELF and the glibc measurement sees
      # nothing new.
      xdg-utils \
      # squashfs tools for the AppImage payload
      squashfs-tools \
      # readelf/objdump — the floor measurement runs inside this image too
      binutils \
      python3 \
    && rm -rf /var/lib/apt/lists/*

# The toolchain is pinned to the workspace's `rust-toolchain.toml` channel.
# It is passed in rather than read from the file because that file also lists
# nine cross targets, and installing all of them to build one host artifact
# costs several gigabytes for nothing.
ARG QANAT_RUST=1.95.0
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/usr/local/node/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --profile minimal \
        --default-toolchain "${QANAT_RUST}" \
        --target x86_64-unknown-linux-gnu \
    && chmod -R a+rwX /opt/rustup /opt/cargo

# Node is needed only to run the Tauri CLI. The frontend is built outside this
# image (`ibni.sh` explains why) and `beforeBuildCommand` is overridden away, so
# nothing here resolves the studio's own dependency tree.
ARG ISDAR_NODE=v24.13.0
RUN curl -fsSL "https://nodejs.org/dist/${ISDAR_NODE}/node-${ISDAR_NODE}-linux-x64.tar.xz" \
      | tar -xJ -C /usr/local \
    && mv "/usr/local/node-${ISDAR_NODE}-linux-x64" /usr/local/node

ARG ISDAR_TAURI=2.11.1
RUN npm install -g "@tauri-apps/cli@${ISDAR_TAURI}" \
    && npm cache clean --force

# linuxdeploy and appimagetool are themselves AppImages, and mounting an
# AppImage needs FUSE, which an unprivileged container does not have. Both tools
# honour this variable and unpack themselves to a temporary directory instead.
ENV APPIMAGE_EXTRACT_AND_RUN=1

WORKDIR /bina
