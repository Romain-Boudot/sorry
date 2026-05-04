#!/usr/bin/env bash
#
# Container entrypoint — builds the Tauri app for the given Rust target.
# Called by scripts/release-desktop.sh; not meant to be invoked directly.
#
# Expects the project root to be mounted at /work, and reads:
#   $TARGET                              (required, e.g. x86_64-pc-windows-msvc)
#   $TAURI_SIGNING_PRIVATE_KEY           (key contents — passed in by host)
#   $TAURI_SIGNING_PRIVATE_KEY_PASSWORD  (optional)
#
set -euo pipefail

TARGET="${1:-${TARGET:?missing TARGET}}"

cd /work/client

# `bun install` has to run inside the container too — the host's node_modules
# might be incompatible (different platform binaries, e.g. esbuild).
bun install --frozen-lockfile

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    bun run tauri build --target "$TARGET"
    ;;
  x86_64-pc-windows-msvc)
    # cargo-xwin downloads the MSVC SDK on first run and caches it.
    bun run tauri build --runner cargo-xwin --target "$TARGET"
    ;;
  *)
    echo "✗ Unsupported target in container: $TARGET" >&2
    exit 1
    ;;
esac

echo "✓ Built $TARGET"
