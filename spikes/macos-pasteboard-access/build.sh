#!/usr/bin/env bash
# Builds KopiSpike.app with swiftc (no Xcode project) and signs it ad hoc.
#
#   ./build.sh            build into ./build/KopiSpike.app
#   BUILD=2 ./build.sh    same, with CFBundleVersion 2 (to mimic an app update)
#   ./build.sh --open     build, then launch with `open`
set -euo pipefail

cd "$(dirname "$0")"
APP="build/KopiSpike.app"
BUNDLE_ID="in.devanshuverma.kopi.spike"
MIN_MACOS="14.0"

rm -rf "$APP" build/obj
mkdir -p "$APP/Contents/MacOS" build/obj

# Universal binary, so the same app can run on an Intel Mac with macOS 14.
for arch in arm64 x86_64; do
  swiftc -O -swift-version 5 \
    -target "$arch-apple-macos$MIN_MACOS" \
    -framework AppKit -framework Security \
    Sources/main.swift -o "build/obj/KopiSpike-$arch"
done
lipo -create build/obj/KopiSpike-arm64 build/obj/KopiSpike-x86_64 -output "$APP/Contents/MacOS/KopiSpike"

cp Info.plist "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${BUILD:-1}" "$APP/Contents/Info.plist"

codesign --force --sign - --identifier "$BUNDLE_ID" "$APP"
codesign --verify --strict "$APP"
codesign -dvvv "$APP" 2>&1 | grep -E '^(Identifier|CDHash|Signature|TeamIdentifier)='

if [[ "${1:-}" == "--open" ]]; then
  open "$APP"
fi
