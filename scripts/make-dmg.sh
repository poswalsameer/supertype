#!/bin/bash
set -euo pipefail
VERSION=${1:-0.2.0}
APP="macos/.build/release/Supertype"
OUT="dist/Supertype-${VERSION}.dmg"
mkdir -p dist
if [ ! -f "$APP" ]; then
  echo "Build release first: swift build --package-path macos -c release"
  exit 1
fi
# Create a folder for DMG contents
TMPDIR=$(mktemp -d)
mkdir -p "$TMPDIR/Supertype"
cp -R "$APP" "$TMPDIR/Supertype/" 2>/dev/null || cp "$APP" "$TMPDIR/Supertype/Supertype"
ln -s /Applications "$TMPDIR/Applications" 2>/dev/null || true
if command -v create-dmg >/dev/null 2>&1; then
  create-dmg --volname "Supertype ${VERSION}" --window-pos 200 120 --window-size 520 300 --icon-size 100 --app-drop-link 380 120 "$OUT" "$TMPDIR/Supertype"
else
  hdiutil create -volname "Supertype ${VERSION}" -srcfolder "$TMPDIR" -ov -format UDZO "$OUT"
fi
rm -rf "$TMPDIR"
echo "DMG at $OUT (codesign/notarize still required with credentials)"
