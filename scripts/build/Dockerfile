# Build environment for the Sorry desktop app.
# - Linux native    (x86_64-unknown-linux-gnu)
# - Windows cross   (x86_64-pc-windows-msvc, via cargo-xwin)
#
# macOS bundles are NOT produced here — Apple's toolchain can't be used in a
# Linux container. Run `tauri build` natively on a Mac for those.
#
# Layer order is intentional: the heaviest cached step (`cargo install
# cargo-xwin`, ~10 min) sits as early as possible so adding/removing Tauri
# Linux deps doesn't bust its cache.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    PATH="/root/.cargo/bin:/root/.bun/bin:$PATH" \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup

# Foundational toolchain (rarely changes).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential curl wget ca-certificates git unzip pkg-config file \
    clang llvm-dev libclang-dev \
 && rm -rf /var/lib/apt/lists/*

# Rust + cross targets.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
 && rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc

# cargo-xwin compiles ~150 crates from source — kept in its own layer so the
# Tauri-deps layer below can churn without rebuilding it.
RUN cargo install --locked cargo-xwin

# Bun (frontend tooling — matches what dev/install scripts use).
RUN curl -fsSL https://bun.sh/install | bash

# Tauri-specific Linux build/runtime deps. Edits to this layer only invalidate
# what comes after, which is just the working dir / entrypoint setup.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libfuse2 \
    xdg-utils desktop-file-utils nsis \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /work

ENTRYPOINT ["/bin/bash", "scripts/build/build-target.sh"]
