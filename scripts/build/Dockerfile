# Build environment for the Sorry desktop app.
# - Linux native    (x86_64-unknown-linux-gnu)
# - Windows cross   (x86_64-pc-windows-msvc, via cargo-xwin)
#
# macOS bundles are NOT produced here — Apple's toolchain can't be used in a
# Linux container. Run `tauri build` natively on a Mac for those.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    PATH="/root/.cargo/bin:/root/.bun/bin:$PATH" \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential curl wget file ca-certificates git unzip pkg-config \
    libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libfuse2 \
    clang llvm-dev libclang-dev nsis \
 && rm -rf /var/lib/apt/lists/*

# Rust + cross targets
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
 && rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc

# Bun (frontend tooling — matches what dev/install scripts use)
RUN curl -fsSL https://bun.sh/install | bash

# Cross-compile to Windows MSVC (downloads MSVC headers/libs on first build,
# cached afterwards in /root/.cache/cargo-xwin which we mount as a volume).
RUN cargo install --locked cargo-xwin

WORKDIR /work

ENTRYPOINT ["/bin/bash", "scripts/build/build-target.sh"]
