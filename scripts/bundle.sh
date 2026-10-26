#!/usr/bin/env bash
# Build a self-contained Rig.app bundle in dist/.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release

APP="dist/Rig.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/rig "$APP/Contents/MacOS/rig"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Rig</string>
  <key>CFBundleDisplayName</key><string>Rig</string>
  <key>CFBundleIdentifier</key><string>dev.rakshak.rig</string>
  <key>CFBundleExecutable</key><string>rig</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc signature: gives the bundle one identity instead of an unsigned binary,
# which is what macOS keys Input Monitoring and Accessibility to.
codesign --force --sign - "$APP" >/dev/null 2>&1 \
  || echo "note: ad-hoc signing failed; the app still runs unsigned"

echo "built $APP"
echo "open it with: open $APP"
