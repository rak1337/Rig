# Changelog

## 1.1.1

- `rig --help` lists every flag, including `--pid`, `--host`, `--token` and
  `--test`, and prints the version
- `Rig.app` takes its version from `Cargo.toml`, so a release is one bump
- the release workflow runs on `macos-latest`, after `macos-14` stopped being
  offered; the stale copy in `deploy/workflows/` is gone
- `actions/checkout@v5` in CI, off the deprecated Node 20 runtime
- the README gains a short release checklist

## 1.1.0

**Look**
- a new mark: a 5 × 5 key grid with the lit keys spelling R, used for the app
  icon, the console header and a new favicon (`/favicon.svg`, also shipped in
  the WebHID build)
- light and dark README banners and a social preview, all rendered by
  `assets/make_brand.py`

**Docs**
- the README is rewritten: quick start, supported hardware, every CLI flag
  (including `--pid`, `--host`, `--token`), architecture and troubleshooting
- it now says plainly that a still look stays on the board after Rig quits;
  only animation and the key features need Rig running

## 1.0.0

First release. Tested against a BlackWidow Tournament Edition Chroma
(`1532:0209`, firmware 1.03) on macOS.

**Lighting**
- 13 effects: static, wave, spectrum, breathing, reactive, fire, starlight,
  ripple, gradient, marquee, comet, typing and off, all rendered by one
  pipeline so a colour means the same thing in every mode
- per-key painting against the board's real 6 × 22 matrix
- brightness, gamma and per-channel gain calibration
- the preview is fed the frame that was actually sent, so it cannot drift

**Keys**
- system-wide remapping through a CGEventTap
- macros with recorded timings, playable from the console or bound to a key
- Hypershift: a second remap layer while a chosen key is held
- global shortcuts: ⌃⌥L lights, ⌃⌥→ next effect, ⌃⌥↑ ↓ brightness

**Around it**
- per-app profiles, presets, idle blanking, gaming mode, macro indicator,
  driver-mode handoff
- state, presets and profiles in `~/Library/Application Support/Rig`
- `--install-login` to start at login and restore the saved look
- `--host` with token auth for the local network, loopback by default
- a server-less WebHID build for lighting only

**Packaging**
- `scripts/bundle.sh` produces `Rig.app` with an icon and a stable bundle id
- CI runs the tests and a release build; tags publish a binary and an app bundle
