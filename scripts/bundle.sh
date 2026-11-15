#!/usr/bin/env bash
# Build Rig.app: a bundle with a stable identity so macOS keeps the permissions
# you granted it, instead of treating every rebuild as a new app.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release

APP="dist/Rig.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/rig "$APP/Contents/MacOS/rig"
cp assets/Rig.icns "$APP/Contents/Resources/Rig.icns"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Rig</string>
  <key>CFBundleDisplayName</key><string>Rig</string>
  <key>CFBundleIdentifier</key><string>dev.rakshak.rig</string>
  <key>CFBundleExecutable</key><string>rig</string>
  <key>CFBundleIconFile</key><string>Rig</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.1.0</string>
  <key>CFBundleVersion</key><string>2</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

codesign --force --sign - "$APP" >/dev/null 2>&1 \
  || echo "note: ad-hoc signing failed; the app still runs unsigned"

echo "built $APP"
echo "open it with: open $APP"
