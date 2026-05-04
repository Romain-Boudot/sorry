#!/usr/bin/env node
/**
 * Build the Tauri updater manifest (latest.json) from the bundles produced by
 * `tauri build`. Reads each `.sig` file, references the matching artifact via
 * its GitHub release download URL, and writes the result to ./latest.json.
 *
 * Usage:
 *   node scripts/build-manifest.mjs <version> [--repo OWNER/NAME] [--notes "..."]
 *
 * Example:
 *   node scripts/build-manifest.mjs 0.1.14 \
 *     --repo Romain-Boudot/sorry \
 *     --notes "Bugfix DM scroll + auto-update support"
 *
 * The script scans `client/src-tauri/target/**\/release/bundle/` for the
 * artifacts Tauri produces and matches them by extension to the right
 * platform key (`darwin-aarch64`, `darwin-x86_64`, `linux-x86_64`,
 * `windows-x86_64`).
 */
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join, basename } from "node:path";
import { argv, exit } from "node:process";

const args = argv.slice(2);
if (!args[0] || args[0].startsWith("-")) {
  console.error("usage: build-manifest.mjs <version> [--repo OWNER/NAME] [--notes \"...\"]");
  exit(1);
}
const version = args[0];
const flags = parseFlags(args.slice(1));
const repo = flags.repo ?? "Romain-Boudot/sorry";
const notes = flags.notes ?? "";

const TARGET_DIR = "client/src-tauri/target";
const releaseUrl = (filename) =>
  `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(filename)}`;

// Tauri produces these bundle types per target. We pick the updater-friendly
// format (the one whose .sig is generated alongside).
//
// Source: https://v2.tauri.app/distribute/sign/updater/
const PLATFORM_RULES = [
  { key: "darwin-aarch64", arch: "aarch64-apple-darwin",       match: /\.app\.tar\.gz$/ },
  { key: "darwin-x86_64",  arch: "x86_64-apple-darwin",        match: /\.app\.tar\.gz$/ },
  { key: "linux-x86_64",   arch: "x86_64-unknown-linux-gnu",   match: /\.AppImage$/ },
  { key: "windows-x86_64", arch: "x86_64-pc-windows-msvc",     match: /\.nsis\.zip$|-setup\.exe\.zip$/ },
];

const platforms = {};

for (const rule of PLATFORM_RULES) {
  const bundleDir = join(TARGET_DIR, rule.arch, "release", "bundle");
  let artifact = null;
  try { artifact = findArtifact(bundleDir, rule.match); }
  catch { /* target not built — skip */ continue; }
  if (!artifact) continue;

  const sigPath = artifact + ".sig";
  let signature;
  try { signature = readFileSync(sigPath, "utf8").trim(); }
  catch {
    console.warn(`! ${rule.key}: bundle present but no .sig (${sigPath}). Did you set TAURI_SIGNING_PRIVATE_KEY at build time?`);
    continue;
  }

  platforms[rule.key] = { signature, url: releaseUrl(basename(artifact)) };
  console.log(`✓ ${rule.key.padEnd(16)} ${basename(artifact)}`);
}

if (Object.keys(platforms).length === 0) {
  console.error("No bundles found. Run `bun run tauri build` first.");
  exit(1);
}

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms,
};

writeFileSync("latest.json", JSON.stringify(manifest, null, 2) + "\n");
console.log(`\n→ Wrote latest.json (v${version}, ${Object.keys(platforms).length} platforms)`);

// ── helpers ──

function findArtifact(dir, regex) {
  // Walk one level deep — Tauri organises bundles by type (macos/, dmg/,
  // appimage/, nsis/...). Pick the first file matching the regex that isn't
  // itself a .sig.
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (!statSync(full).isDirectory()) continue;
    for (const file of readdirSync(full)) {
      if (file.endsWith(".sig")) continue;
      if (regex.test(file)) return join(full, file);
    }
  }
  return null;
}

function parseFlags(rest) {
  const out = {};
  for (let i = 0; i < rest.length; i++) {
    const f = rest[i];
    if (f === "--repo")  out.repo  = rest[++i];
    if (f === "--notes") out.notes = rest[++i];
  }
  return out;
}
