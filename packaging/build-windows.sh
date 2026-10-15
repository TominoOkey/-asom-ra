#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
VERSION=$(awk -F '"' '/^version = / { print $2; exit }' Cargo.toml)
TARGET=x86_64-pc-windows-msvc

if [ -z "$VERSION" ]; then
    echo "Nelze zjistit verzi z Cargo.toml" >&2
    exit 1
fi
if ! command -v cargo-xwin >/dev/null 2>&1; then
    echo "Chybí cargo-xwin. Nainstaluj jej příkazem: cargo install cargo-xwin" >&2
    exit 1
fi
if ! command -v makensis >/dev/null 2>&1; then
    echo "Chybí NSIS. Na macOS jej nainstaluj příkazem: brew install nsis" >&2
    exit 1
fi

RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
    cargo xwin build --release --target "$TARGET" --bin casomira
mkdir -p dist/windows
cp "target/$TARGET/release/casomira.exe" "dist/windows/Casomira-$VERSION-windows-x64.exe"
cp packaging/windows/README.txt dist/windows/README.txt
rm -f "dist/windows/Casomira-$VERSION-windows-x64.zip"
zip -j "dist/windows/Casomira-$VERSION-windows-x64.zip" \
    "dist/windows/Casomira-$VERSION-windows-x64.exe" dist/windows/README.txt
makensis -V2 -DAppVersion="$VERSION" packaging/windows/Casomira.nsi
echo "Hotovo: dist/windows/Casomira-Setup-$VERSION.exe a Casomira-$VERSION-windows-x64.exe"
