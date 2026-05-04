# Desktop releases & auto-update

The desktop app uses Tauri's updater plugin. Releases are built locally and
uploaded to GitHub Releases. The desktop app fetches the manifest from the
**latest** release on every startup.

| Target            | Built from           | Notes                                |
|-------------------|----------------------|--------------------------------------|
| macOS (universal) | macOS only — native  | Apple toolchain can't run elsewhere  |
| Linux x86_64      | Docker / Podman      | Any host (Mac, Windows, Linux)       |
| Windows x86_64    | Docker / Podman      | Cross-compile via `cargo-xwin`       |

The release script builds whatever the host can produce. Targets you skip just
won't appear in `latest.json`, so users on those platforms keep their current
version until you ship the missing target.

## One-time setup

### 1. Generate the updater signing key

ED25519 keypair used to sign update bundles. The **public** key gets embedded in
the app at build time; the **private** key stays on your machine (and gets
exported as an env var when releasing).

```bash
mkdir -p ~/.tauri
cd client && bun run tauri signer generate -w ~/.tauri/sorry.key
# Prints the public key — copy it.
```

Then paste the public key into [client/src-tauri/tauri.conf.json](../client/src-tauri/tauri.conf.json):

```json
"plugins": {
  "updater": {
    "endpoints": ["https://github.com/Romain-Boudot/sorry/releases/latest/download/latest.json"],
    "pubkey": "<paste here>"
  }
}
```

Commit that change. **Never** commit the private key (`*.key` is gitignored).

### 2. Where the release script looks for the key

[scripts/release-desktop.sh](../scripts/release-desktop.sh) resolves the signing
key in this order:

1. `$TAURI_SIGNING_PRIVATE_KEY` if exported (file path OR raw contents)
2. `~/.tauri/sorry.key` if it exists on disk
3. **Interactive prompt** — paste the contents from your password manager,
   then press Enter on an empty line to validate. Used when the key is not
   stored on the build machine.

The interactive prompt is the recommended path if you keep the key in a vault
(1Password, Bitwarden, etc.) — it never touches disk between releases.

### 3. Install Rust targets (macOS only)

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

Linux + Windows builds run inside a container — no host Rust toolchain
required for those.

### 4. GitHub CLI

```bash
# macOS
brew install gh
# Windows
winget install --id GitHub.cli
# Linux
# see https://cli.github.com/

gh auth login
```

### 5. Container engine (for Linux / Windows builds)

Docker Desktop or Podman, with the daemon running. Either works — the script
auto-detects.

```bash
# macOS
brew install --cask docker
# Windows
winget install Docker.DockerDesktop
# Linux
# see https://docs.docker.com/engine/install/
```

## Cutting a release

```bash
# 1. Bump version across the 3 files in lockstep.
scripts/bump-version.sh 0.1.14

# 2. Commit + push.
git commit -am "chore: release v0.1.14" && git push

# 3. Build + manifest + publish.
scripts/release-desktop.sh 0.1.14 --notes "DM consistency + auto-update"
```

The release script:
1. **macOS native** (only on Mac) — `tauri build --target universal-apple-darwin`
2. **Linux** in a container — `tauri build --target x86_64-unknown-linux-gnu`
3. **Windows** in a container — `tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`
4. Generates `latest.json` from whatever bundles it produced via [scripts/build-manifest.mjs](../scripts/build-manifest.mjs)
5. Uploads bundles + `latest.json` to the GitHub release tagged `v<version>`

The first run on a new machine takes longer because Docker pulls the base image
and builds the Tauri image (`sorry-builder`), and `cargo-xwin` downloads the
MSVC SDK. Subsequent runs are fast — both are cached in a named volume
(`sorry-builder-cache`).

## Skipping targets

Each target has an opt-out env var:

```bash
BUILD_WINDOWS=0 scripts/release-desktop.sh 0.1.14 --notes "..."
BUILD_LINUX=0 BUILD_WINDOWS=0 scripts/release-desktop.sh 0.1.14 --notes "macOS hotfix"
```

Common scenarios:
- **No container engine available** → `BUILD_LINUX=0 BUILD_WINDOWS=0` (only macOS native, on Mac)
- **Re-releasing macOS later from your Mac** after a Windows-host release →
  the script uploads with `--clobber`, so you can append macOS bundles to an
  existing release. Just run again on the Mac with whatever targets you skip.

## What the desktop app does

- On boot, [useUpdater.ts](../client/src/composables/useUpdater.ts) fetches the
  manifest from the configured endpoint (silent — only sets state).
- If a newer version is available, [App.vue](../client/src/App.vue) renders a
  banner at the top: "Mise a jour disponible — Installer / Plus tard".
- Settings → A propos has a "Verifier maintenant" button for manual checks.
- Install path: `download → verify ED25519 sig → apply → relaunch`.

## OS code signing (separate from updater signing)

The updater signature proves "this bundle came from us". OS code signing
(Apple notarization, Windows code-sign cert) is a separate concern: it stops
Gatekeeper / SmartScreen warnings at *install* time but is **not required**
for the auto-updater to work.

When ready:

- macOS: export `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`,
  `APPLE_TEAM_ID`. Tauri will sign + notarize automatically during
  `tauri build`.
- Windows: requires a code-signing cert ($200-400/year) and additional config
  in `tauri.conf.json`. Not free, not in scope here.

## Troubleshooting

**"signature mismatch" at install time** — public key in `tauri.conf.json`
doesn't match the private key used to sign the bundle. Verify
`TAURI_SIGNING_PRIVATE_KEY` was exported when the *currently running* app was
built, not just when releasing.

**Updater never fires** — open DevTools (Tauri builds expose it in dev). Check
network tab for the manifest fetch. The endpoint must be reachable and return
200. GitHub Releases takes ~30 seconds after upload to be reachable on the
`latest` redirect.

**Bundle missing from manifest** — `build-manifest.mjs` only includes targets
whose `.sig` file exists. If a target was skipped during build, the manifest
won't reference it (other platforms still get updates).

**Container build fails on first run** — `cargo-xwin` needs to download the
MSVC SDK from Microsoft (~1 GB). First Windows build takes 5-10 minutes. If
your network blocks it, set `BUILD_WINDOWS=0` and ship Windows from a Windows
host using `tauri build` natively.

**Volume/permission errors with Podman rootless** — Podman in rootless mode
needs `:Z` SELinux label or `--userns=keep-id` to bind-mount cleanly. Easiest
fix: set `ENGINE=docker` if you have both, or run Podman with the rootful
socket.
