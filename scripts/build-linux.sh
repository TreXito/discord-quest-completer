#!/bin/sh
# Build the Linux app: runner -> resources -> Tauri bundles (deb + AppImage).
#
#   ./scripts/build-linux.sh            # same as: pnpm build:linux
#
# Why this wrapper exists: on distros with newer binutils (Fedora 41+, i.e. any
# glibc built with .relr.dyn sections) linuxdeploy's bundled `strip` cannot parse
# the system libraries it copies into the AppDir, so the AppImage step dies with
#
#   ERROR: Strip call failed: ...: unknown type [0x13] section `.relr.dyn'
#   failed to bundle project: `failed to run linuxdeploy`
#
# The .deb and the plain binary are built fine in that case; only the AppImage
# fails. linuxdeploy honours NO_STRIP=1, so retry just the bundle step with it.
# NO_STRIP leaves the bundled libs unstripped (bigger AppImage, still functional).
set -eu

cd "$(dirname "$0")/.."

pnpm build:runner:linux
pnpm copy:runner:linux

if pnpm exec tauri build "$@"; then
    exit 0
fi

echo "" >&2
echo "tauri build failed - retrying the bundle with NO_STRIP=1 (newer binutils," >&2
echo "linuxdeploy's strip cannot handle .relr.dyn; see the comment in this script)." >&2
echo "" >&2

NO_STRIP=1 pnpm exec tauri build "$@"
