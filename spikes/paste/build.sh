#!/bin/sh
# Builds build/KopiPasteSpike.app. Needs Rust (rustup) and the Command Line Tools, not the Tauri CLI.
#
#   ./build.sh            build
#   ./build.sh --open     build, then launch
#
# Each build gets a new ad-hoc signature, so macOS forgets the Accessibility grant for the old one.
set -eu
cd "$(dirname "$0")"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

# tauri/custom-protocol embeds ui/ in the binary, as `tauri build` would.
cargo build --release --features tauri/custom-protocol

APP=build/KopiPasteSpike.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/kopi-paste-spike "$APP/Contents/MacOS/"
cp Info.plist "$APP/Contents/Info.plist"
codesign --force --sign - "$APP"
echo "Built $APP"

if [ "${1:-}" = "--open" ]; then
  pkill -x kopi-paste-spike 2>/dev/null || true
  open "$APP"
fi
