#!/usr/bin/env bash
#
# Release the Sorry desktop app:
#   - Builds the targets that can be built on the current host:
#       · macOS universal  → native, only when running on macOS
#       · Linux x86_64     → via Docker/Podman container
#       · Windows x86_64   → via Docker/Podman container (cross-compile)
#   - Generates the signed updater manifest (latest.json)
#   - Uploads bundles + manifest to a GitHub release
#
# Prereqs:
#   - bun + Rust installed for the macOS native build (Mac only)
#   - docker OR podman available, with their daemon running, for Linux/Windows
#   - gh CLI authenticated (gh auth login)
#   - ED25519 private signing key. Resolved in this order:
#       1. $TAURI_SIGNING_PRIVATE_KEY if exported (file path OR raw contents)
#       2. ~/.tauri/sorry.key if it exists on disk
#       3. Interactive prompt — paste contents from your password manager
#   - Optional: $TAURI_SIGNING_PRIVATE_KEY_PASSWORD if your key has a passphrase
#
# Per-target opt-out via env vars: BUILD_MACOS=0, BUILD_LINUX=0, BUILD_WINDOWS=0
#
# Usage:
#   scripts/release-desktop.sh <version> [--notes "release notes"]
#
set -euo pipefail

# ── Args ──
if [ $# -lt 1 ] || [[ "$1" == --* ]]; then
  echo "usage: $0 <version> [--notes \"...\"]" >&2
  exit 1
fi
VERSION="$1"; shift

NOTES=""
while [ $# -gt 0 ]; do
  case "$1" in
    --notes) NOTES="$2"; shift 2 ;;
    *) echo "unknown flag: $1" >&2; exit 1 ;;
  esac
done

REPO="${REPO:-Romain-Boudot/sorry}"
TAG="v$VERSION"

# Default targets: build everything the host can produce.
HOST_OS="$(uname -s)"
case "$HOST_OS" in
  Darwin) BUILD_MACOS="${BUILD_MACOS:-1}" ;;
  *)      BUILD_MACOS="${BUILD_MACOS:-0}" ;;  # Apple toolchain only on Mac
esac
BUILD_LINUX="${BUILD_LINUX:-1}"
BUILD_WINDOWS="${BUILD_WINDOWS:-1}"

cd "$(dirname "$0")/.."

# ── Sanity ──
if ! command -v gh >/dev/null; then
  echo "✗ gh CLI not found." >&2
  echo "  macOS: brew install gh" >&2
  echo "  Windows: winget install --id GitHub.cli" >&2
  echo "  Linux: see https://cli.github.com/" >&2
  exit 1
fi

# Detect container engine for Linux/Windows builds.
ENGINE=""
if [ "$BUILD_LINUX" = "1" ] || [ "$BUILD_WINDOWS" = "1" ]; then
  if command -v docker >/dev/null; then
    ENGINE="docker"
  elif command -v podman >/dev/null; then
    ENGINE="podman"
  else
    echo "✗ Neither docker nor podman available — cannot do containerized builds." >&2
    echo "  Skip them with: BUILD_LINUX=0 BUILD_WINDOWS=0 $0 $VERSION" >&2
    exit 1
  fi
fi

# ── Version sanity ──
CONF_VERSION=$(grep -m1 '"version"' client/src-tauri/tauri.conf.json | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
if [ "$CONF_VERSION" != "$VERSION" ]; then
  echo "⚠  tauri.conf.json says v$CONF_VERSION but releasing v$VERSION."
  echo "   Run scripts/bump-version.sh $VERSION first, then commit, then re-run."
  echo "   Continue anyway? [y/N]"
  read -r ans
  [[ "$ans" =~ ^[yY]$ ]] || exit 1
fi

# ── Signing key resolution ──
DEFAULT_KEY_PATH="$HOME/.tauri/sorry.key"
if [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  : # use as-is
elif [ -f "$DEFAULT_KEY_PATH" ]; then
  export TAURI_SIGNING_PRIVATE_KEY="$DEFAULT_KEY_PATH"
else
  echo ""
  echo "→ Private signing key needed. Paste it from your password manager,"
  echo "  then press Enter on an empty line to validate."
  echo ""
  KEY_INPUT=""
  while IFS= read -r line; do
    if [ -z "$line" ]; then
      [ -n "$KEY_INPUT" ] && break
      continue
    fi
    KEY_INPUT="${KEY_INPUT}${line}"$'\n'
  done
  if [ -z "$KEY_INPUT" ]; then
    echo "✗ Empty key — aborting." >&2
    exit 1
  fi
  export TAURI_SIGNING_PRIVATE_KEY="$KEY_INPUT"
  echo "✓ Key loaded ($(echo "$KEY_INPUT" | wc -l | tr -d ' ') line(s))."
  unset KEY_INPUT
fi

# Passphrase prompt is independent of how the key was acquired — even a key
# from disk or env var may still be encrypted. Skip only if explicitly set.
if [ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+set}" ]; then
  echo ""
  read -rs -p "→ Key passphrase (empty if the key has none): " PW
  echo ""
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$PW"
  unset PW
fi

# Containers receive the key as a single base64 line. Tauri's env-var path
# does NOT parse the minisign file format — passing the comment header or any
# embedded newlines makes the base64 decode fail mid-string.
RAW_KEY="$TAURI_SIGNING_PRIVATE_KEY"
if [ -f "$RAW_KEY" ]; then
  RAW_KEY="$(cat "$RAW_KEY")"
fi
# Drop the optional `untrusted comment:` header line and strip every byte of
# whitespace/CR/LF from the rest, leaving only the base64 payload.
KEY_FOR_CONTAINER="$(printf '%s' "$RAW_KEY" \
  | awk '!/^untrusted comment:/' \
  | tr -d '[:space:]')"
unset RAW_KEY
if [ -z "$KEY_FOR_CONTAINER" ]; then
  echo "✗ Failed to extract base64 key from TAURI_SIGNING_PRIVATE_KEY contents." >&2
  exit 1
fi

# ── Build container image (idempotent — uses build cache) ──
if [ -n "$ENGINE" ]; then
  echo ""
  echo "━━ Building $ENGINE image (sorry-builder) ━━"
  $ENGINE build -f scripts/build/Dockerfile -t sorry-builder .
fi

# Volume name for caches (xwin SDK, cargo registry).
CACHE_VOL="sorry-builder-cache"
if [ -n "$ENGINE" ]; then
  $ENGINE volume create "$CACHE_VOL" >/dev/null 2>&1 || true
fi

# Resolve a host path that container engines on Windows accept. On Git-Bash /
# MSYS2, $PWD looks like `/c/Users/...` — engines expect `C:\Users\...`.
host_path() {
  if command -v cygpath >/dev/null; then
    cygpath -w "$1"
  else
    echo "$1"
  fi
}

run_container() {
  local target="$1"
  echo ""
  echo "━━ Building $target via $ENGINE ━━"
  # MSYS_NO_PATHCONV=1 stops Git-Bash from rewriting destination paths like
  # `/work` and `/root/.cache` when passing them to a native Windows engine.
  MSYS_NO_PATHCONV=1 $ENGINE run --rm \
    -v "$(host_path "$PWD"):/work" \
    -v "$CACHE_VOL:/root/.cache" \
    -e "TARGET=$target" \
    -e "TAURI_SIGNING_PRIVATE_KEY=$KEY_FOR_CONTAINER" \
    -e "TAURI_SIGNING_PRIVATE_KEY_PASSWORD=${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" \
    sorry-builder "$target"
}

# ── macOS native build ──
if [ "$BUILD_MACOS" = "1" ]; then
  if [ "$HOST_OS" != "Darwin" ]; then
    echo "⚠  BUILD_MACOS=1 but host is $HOST_OS — Apple toolchain unavailable. Skipping."
  else
    echo ""
    echo "━━ Building macOS (universal) ━━"
    ( cd client && bun install --frozen-lockfile )
    ( cd client && bun run tauri build --target universal-apple-darwin )
    # Symlink universal output under both arch directories so the manifest
    # script picks it up for both darwin-aarch64 and darwin-x86_64.
    for arch in aarch64-apple-darwin x86_64-apple-darwin; do
      mkdir -p "target/$arch/release"
      rm -rf "target/$arch/release/bundle"
      ln -s "../../universal-apple-darwin/release/bundle" \
            "target/$arch/release/bundle"
    done
  fi
fi

# ── Linux build ──
if [ "$BUILD_LINUX" = "1" ]; then
  run_container x86_64-unknown-linux-gnu
fi

# ── Windows cross-compile ──
if [ "$BUILD_WINDOWS" = "1" ]; then
  run_container x86_64-pc-windows-msvc
fi

# ── Manifest ──
echo ""
echo "━━ Building latest.json ━━"
node scripts/build-manifest.mjs "$VERSION" --repo "$REPO" --notes "$NOTES"

# ── Collect artefacts ──
echo ""
echo "━━ Collecting artefacts ━━"
mkdir -p dist-release
rm -f dist-release/*

# Copy a glob into dist-release/, silently ignoring "no match" via shopt nullglob.
shopt -s nullglob
copy_glob() {
  local files=( $1 )
  if [ ${#files[@]} -gt 0 ]; then
    cp "${files[@]}" dist-release/
    for f in "${files[@]}"; do echo "  + $f"; done
  fi
}

if [ "$BUILD_MACOS" = "1" ] && [ "$HOST_OS" = "Darwin" ]; then
  copy_glob "target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz"
  copy_glob "target/universal-apple-darwin/release/bundle/macos/*.app.tar.gz.sig"
fi
if [ "$BUILD_LINUX" = "1" ]; then
  copy_glob "target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage"
  copy_glob "target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage.tar.gz"
  copy_glob "target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage.sig"
  copy_glob "target/x86_64-unknown-linux-gnu/release/bundle/appimage/*.AppImage.tar.gz.sig"
fi
if [ "$BUILD_WINDOWS" = "1" ]; then
  # Tauri 2 current: signed -setup.exe (.exe.sig). Older: .nsis.zip (+ .sig).
  copy_glob "target/x86_64-pc-windows-msvc/release/bundle/nsis/*-setup.exe"
  copy_glob "target/x86_64-pc-windows-msvc/release/bundle/nsis/*-setup.exe.sig"
  copy_glob "target/x86_64-pc-windows-msvc/release/bundle/nsis/*.nsis.zip"
  copy_glob "target/x86_64-pc-windows-msvc/release/bundle/nsis/*.nsis.zip.sig"
fi
shopt -u nullglob

cp latest.json dist-release/

# Sanity: at least one bundle (.sig present) must have been collected, otherwise
# the upload will be useless to the auto-updater.
if ! ls dist-release/*.sig >/dev/null 2>&1; then
  echo "✗ No signed bundles in dist-release/. Did the build produce .sig files?" >&2
  echo "  Inspect: target/<arch>/release/bundle/" >&2
  exit 1
fi

echo ""
echo "Final dist-release/ contents:"
ls -lh dist-release/

# ── Publish ──
echo ""
echo "━━ Creating GitHub release $TAG ━━"
if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  echo "Release $TAG already exists — uploading bundles to it."
  gh release upload "$TAG" dist-release/* --repo "$REPO" --clobber
else
  gh release create "$TAG" \
    --repo "$REPO" \
    --title "$TAG" \
    --notes "$NOTES" \
    dist-release/*
fi

echo ""
echo "✓ Done. Updater endpoint: https://github.com/$REPO/releases/latest/download/latest.json"
