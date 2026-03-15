#!/usr/bin/env bash
# Generate offline dependency sources for Flathub Flatpak builds.
#
# Prerequisites:
#   pip install flatpak-builder-tools         # provides both generators
#   — OR install from https://github.com/nickvergessen/flatpak-builder-tools
#
# Usage:
#   cd <repo-root>/flatpak
#   ./generate-sources.sh
#
# This produces:
#   generated-sources/cargo-sources.json   — Cargo crate downloads
#   generated-sources/node-sources.json    — npm tarball downloads
#
# Commit these files alongside the manifest when submitting to Flathub.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
OUT_DIR="$SCRIPT_DIR/generated-sources"

mkdir -p "$OUT_DIR"

echo "==> Generating Cargo sources from src-tauri/Cargo.lock ..."
python3 -m flatpak_cargo_generator \
    "$REPO_ROOT/src-tauri/Cargo.lock" \
    -o "$OUT_DIR/cargo-sources.json"
echo "    -> $OUT_DIR/cargo-sources.json"

echo "==> Generating Node sources from package-lock.json ..."
flatpak-node-generator npm \
    "$REPO_ROOT/package-lock.json" \
    -o "$OUT_DIR/node-sources.json"
echo "    -> $OUT_DIR/node-sources.json"

echo ""
echo "Done. Before submitting to Flathub:"
echo "  1. Update the git tag and commit hash in de.carpeasrael.stichman.yml"
echo "  2. Add screenshots to screenshots/ and update metainfo.xml"
echo "  3. Validate with:"
echo "     flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest de.carpeasrael.stichman.yml"
echo "     flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream de.carpeasrael.stichman.metainfo.xml"
