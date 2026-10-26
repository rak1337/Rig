#!/usr/bin/env bash
# Assemble the static, server-less build: lighting over WebHID.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="dist-webhid"
rm -rf "$OUT"
mkdir -p "$OUT/fonts"

cp webhid/index.html "$OUT/index.html"
cp webhid/hid.js "$OUT/hid.js"
cp web/app.css "$OUT/app.css"
cp web/fonts/Archivo.ttf "$OUT/fonts/archivo.ttf"
cp web/fonts/IBMPlexMono.ttf "$OUT/fonts/plex-mono.ttf"

cat > "$OUT/wrangler.jsonc" <<'JSONC'
{
  "name": "rig",
  "compatibility_date": "2026-10-26",
  "assets": {
    "directory": "./",
    "not_found_handling": "single-page-application"
  },
  "routes": [{ "pattern": "rig.azmuth.dev", "custom_domain": true }]
}
JSONC

echo "built $OUT"
echo "preview: python3 -m http.server 8080 --directory $OUT"
echo "deploy:  npx wrangler deploy --cwd $OUT"
