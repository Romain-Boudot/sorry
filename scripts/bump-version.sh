#!/usr/bin/env bash
#
# Bump the project version in lockstep across the three files that hold it:
#   - client/src-tauri/tauri.conf.json (drives the Tauri updater comparison)
#   - Cargo.toml (workspace.package.version — server + tauri-lib + shared)
#   - client/package.json (npm metadata)
#
# Usage:
#   scripts/bump-version.sh 0.1.14
#
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <version>" >&2
  exit 1
fi
VERSION="$1"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "✗ Invalid version: $VERSION (expected X.Y.Z, optionally with -prerelease or +build)" >&2
  exit 1
fi

cd "$(dirname "$0")/.."

# Portable in-place sed (BSD/macOS + GNU/Linux): write to .bak then drop it.
sedi() { sed -i.bak "$@"; rm -f "${@: -1}.bak"; }

# 1. tauri.conf.json — root-level "version": "..."
TAURI_CONF="client/src-tauri/tauri.conf.json"
sedi -E 's/^([[:space:]]*"version"[[:space:]]*:[[:space:]]*")[^"]+(")/\1'"$VERSION"'\2/' "$TAURI_CONF"

# 2. Cargo.toml — version inside the [workspace.package] section only.
sedi -E '/^\[workspace\.package\]/,/^\[/ s/^version[[:space:]]*=[[:space:]]*"[^"]+"/version = "'"$VERSION"'"/' Cargo.toml

# 3. client/package.json — root-level "version": "..."
sedi -E 's/^([[:space:]]*"version"[[:space:]]*:[[:space:]]*")[^"]+(")/\1'"$VERSION"'\2/' client/package.json

# Verify
NEW_TAURI=$(grep -m1 '"version"' "$TAURI_CONF" | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
NEW_CARGO=$(awk '/^\[workspace\.package\]/{p=1; next} /^\[/{p=0} p && /^version[[:space:]]*=/{print; exit}' Cargo.toml \
  | sed -E 's/.*"([^"]+)".*/\1/')
NEW_PKG=$(grep -m1 '"version"' client/package.json | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')

if [ "$NEW_TAURI" != "$VERSION" ] || [ "$NEW_CARGO" != "$VERSION" ] || [ "$NEW_PKG" != "$VERSION" ]; then
  echo "✗ Bump failed — files not in sync after sed:" >&2
  echo "    tauri.conf.json: $NEW_TAURI" >&2
  echo "    Cargo.toml:      $NEW_CARGO" >&2
  echo "    package.json:    $NEW_PKG" >&2
  exit 1
fi

echo "✓ Bumped to $VERSION:"
echo "  - $TAURI_CONF"
echo "  - Cargo.toml (workspace.package)"
echo "  - client/package.json"
