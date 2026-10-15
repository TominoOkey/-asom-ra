#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

VERSION=$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)
APP="dist/macos/Časomíra.app"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

if [ -z "$VERSION" ]; then
    echo "Nelze zjistit verzi z Cargo.toml" >&2
    exit 1
fi

mkdir -p "$MACOS" "$RESOURCES"
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/casomira "$MACOS/casomira"
chmod +x "$MACOS/casomira"
cat > "$CONTENTS/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>casomira</string>
<key>CFBundleIdentifier</key><string>cz.casomira.desktop</string>
<key>CFBundleName</key><string>Časomíra</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$VERSION</string>
<key>CFBundleVersion</key><string>$VERSION</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF

mkdir -p dist/macos
hdiutil create -volname "Časomíra" -srcfolder "$APP" -ov -format UDZO "dist/macos/Casomira-$VERSION-aarch64.dmg"
echo "Hotovo: dist/macos/Casomira-$VERSION-aarch64.dmg"
