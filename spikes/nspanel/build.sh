#!/bin/sh
# Builds build/KopiPanelSpike.app. Needs Rust (rustup) and the Command Line Tools, not the Tauri CLI.
#
#   ./build.sh            build
#   ./build.sh --open     build, then launch
#   LSUI=1 ./build.sh     also set LSUIElement in Info.plist (no Dock icon from the first frame)
set -eu
cd "$(dirname "$0")"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

# tauri/custom-protocol embeds ui/ in the binary, as `tauri build` would.
cargo build --release --features tauri/custom-protocol

APP=build/KopiPanelSpike.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/kopi-panel-spike "$APP/Contents/MacOS/"
cp Info.plist "$APP/Contents/Info.plist"
if [ "${LSUI:-0}" = 1 ]; then
  /usr/libexec/PlistBuddy -c "Add :LSUIElement bool true" "$APP/Contents/Info.plist"
fi
codesign --force --sign - "$APP"
echo "Built $APP (LSUIElement: ${LSUI:-0})"

if [ "${1:-}" = "--open" ]; then
  pkill -x kopi-panel-spike 2>/dev/null || true
  open "$APP"
fi
