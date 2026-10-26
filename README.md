# Rig

A control console for Razer keyboards on macOS: lighting, per-key frames, presets
and device settings, in one place. It talks to the keyboard over USB directly — no
Razer Synapse, no kernel extension, no Electron — and serves its own interface in
the browser.

It exists because Razer never shipped working macOS support for legacy Chroma
hardware. The Synapse build available for this board on macOS is the old
Synapse 2-era client (`0.8.10`), whose bundle contains no device database at all:
no entry for `1532:0209`, no "BlackWidow" string anywhere. The protocol, however,
was documented years ago by the OpenRazer project, and it is portable.

## Verified on hardware

Razer BlackWidow Tournament Edition Chroma, `1532:0209`, firmware `1.03`.
Every command below returns status `0x02` (successful) from the device.

- **Device effects** — Static, Wave (left/right), Spectrum, Breathing (one colour,
  two colours, random), Reactive (speed 1–4), Off
- **Rendered effects** — Fire, Starlight, Ripple, Gradient, Marquee, Comet. These
  are computed on the host and pushed as custom frames at ~40 fps, the same way
  Razer's own software animates this board.
- **Brightness** — 0–100, mapped to the device's 0–255 level
- **Custom frames** — paint individual keys in the 6 × 22 LED matrix using the
  board's real key map, or fill the whole board
- **Presets** — save the current look, apply it later, delete it
- **Gaming mode** — locks the Windows key, lights the game indicator
- **Macro indicator** — toggles the macro LED
- **Driver mode** — hand the LEDs back to the keyboard's firmware, or take them over
- **Restore on launch** — the last state is re-applied when the service starts

## Build and run

```sh
cargo build --release
./target/release/rig
```

It listens on `http://127.0.0.1:7311` and opens the console in your browser.

```sh
rig [--port 7311] [--no-open] [--no-restore]
rig --probe               # read the board's current state and exit
rig --install-login       # start at login, re-apply the saved look
rig --uninstall-login
rig --test                # round-trip a report to the keyboard and exit
```

State and presets live in `~/Library/Application Support/Rig/state.json`.

## Remapping and macros

The **Keys** tab remaps keys and records macros system-wide through a
`CGEventTap`, so both work in every app, not just in this window.

- **Remap** — press the key to change, then press what it should act as. Stored in
  `state.json`, applied at launch.
- **Macro** — Record captures key down/up with timings until you stop; Play replays
  them. Playback events are tagged so the tap does not remap its own output.

Both need **Accessibility** permission (System Settings → Privacy & Security →
Accessibility). Without it the tap is never created and the tab says so.

## Compared with Razer Synapse

| Capability | Here | Notes |
| --- | --- | --- |
| Device effects (wave, spectrum, breathing, reactive, static, off) | yes | runs in the keyboard's firmware |
| Fire, starlight, ripple, gradient, marquee, comet, typing | yes | rendered here, ~40 fps |
| Per-key painting | yes | real 6 × 22 key map |
| Brightness, gaming mode, macro LED, driver mode | yes | |
| Key remapping | yes | system-wide event tap |
| Macros with trigger keys | yes | record, play, bind to a key |
| Presets | yes | local, restored at launch |
| Per-app profiles | yes | switches look and remaps with the frontmost app |
| Hypershift / Fn layer | yes | hold a chosen key for a second remap set |
| Audio visualiser | not yet | needs audio capture plus an FFT |
| Chroma Connect / game integrations | no | Razer's Windows SDK |
| On-board profiles | no | this board has no profile storage |
| Polling rate | no | fixed in firmware |

## Profiles and Hypershift

**App profiles** capture the current look *and* remaps. Bind a profile to an app
and it applies whenever that app comes forward — the watcher reads the frontmost
bundle identifier once a second and switches when it changes. The console polls a
revision counter, so a switch made by the watcher shows up in the UI too.

**Hypershift** is a second remap table gated by a held key: choose a key under
Keys → Hypershift, fill layer 2, and holding that key swaps the remap table while
it is down. The tap tracks the key's press state itself, so it works with any key
you pick.

## Layouts and devices

The service reports which keyboard it found (`src/devices.rs`), including a layout
id, matrix size and whether per-key painting is possible. The console draws the
layout it is told to draw; unknown boards get a generic ANSI TKL rather than a
wrong specific one, with painting disabled. `--pid 0x0203` points the app at
another board in the table.

The drawing follows the hardware: a matte case with a raised deck and a front lip,
sharp-cornered caps with wells between them, legends aligned to the top-left of
each cap the way Razer prints them, shifted symbols stacked under the number row,
light bleeding from under each cap and lighting the legends, and the emblem
centred on the lip.

## What the board actually holds

```sh
rig --probe
```

prints what the keyboard reports about itself: firmware, whether software or
firmware currently owns the LEDs, the stored brightness, and the gaming/macro
indicator states.

There is no profile table to read. Razer's profiles are a Synapse-side (Windows)
concept, and this board keeps no named on-board profiles. What it can hold is the
single effect currently applied, and only while it has power — Chroma effects on
this generation are host-driven, so the host sends every frame.

That means "save the config to the keyboard" is not possible on a TE Chroma. The
durable equivalent is to re-apply the look on each host:

```sh
rig --install-login    # LaunchAgent: start at login, restore the saved look
rig --uninstall-login
```

The login item starts the service at login and re-applies
`~/Library/Application Support/Rig/state.json`, so macOS comes back to your
look without you touching it. On Windows, Synapse does the same job. Whichever
machine applies last wins while both are connected.

## Running it for other devices

By default the console binds to `127.0.0.1`, so only this machine can reach it.
To drive the keyboard from a phone or another computer on the same network:

```sh
./target/release/rig --host 0.0.0.0
rig listening on http://0.0.0.0:7311
open this to sign in: http://0.0.0.0:7311/?t=<token>
```

A token is generated whenever the bind address is not loopback, because this
service drives real hardware. Opening the printed link trades the token for a
cookie and redirects, so it leaves the address bar. `--token` or `RIG_TOKEN`
sets your own. The page is plain HTTP: keep it on a trusted network, or put it
behind Tailscale / a reverse proxy with TLS. Do not port-forward it to the
internet.

## Global shortcuts

| Keys | Action |
| --- | --- |
| ⌃⌥L | lights off / back on |
| ⌃⌥→ | next effect |
| ⌃⌥↑ / ⌃⌥↓ | brightness up / down |

These are handled by the same event tap as remapping, so they need Accessibility
permission and work in any app.

## Idle and backups

**Dim when idle** (Device tab) blanks the backlight after N minutes without
keyboard or mouse input and restores it on the next input. Zero disables it.

**Backup** exports presets, profiles, remaps, macros and calibration as one JSON
file and imports it again, so a setup moves between machines.

## Tests

```sh
cargo test
```

Covers the frame renderer (every effect fills the matrix, static is exact,
gamma and gain behave), the report format (90-byte layout, checksum, frame
sizing, brightness mapping) and the keycode-to-cell map. None of it needs the
keyboard attached, and CI runs it on every push.

## Permission

macOS gates keyboard HID access behind **Input Monitoring**. On first run the app
asks for it; if it is missing you will see `macOS Input Monitoring permission is
required`. Grant it in System Settings → Privacy & Security → Input Monitoring for
the terminal (or app) you launch `rig` from, then run it again.

## How it works

A browser page cannot send USB feature reports, so the native side is a small
Rust service and the UI is a page it serves:

```
src/hid.rs      IOKit HID manager, feature report exchange, TCC check
src/report.rs   Razer report construction: header, checksum, effects, frames
src/main.rs     HTTP server, JSON API, persisted state, startup restore
web/            the console (HTML/CSS/JS, embedded into the binary)
```

Everything in `web/` is compiled into the binary with `include_str!` /
`include_bytes!`, so the app ships as one file with no runtime assets.

### The report format

A 90-byte vendor feature report, addressed to report ID 0:

```
byte 0      status (0x00 new command, 0x02 successful)
byte 1      transaction id (0xFF for this generation)
bytes 2-3   remaining packets, big endian
byte 4      protocol type
byte 5      data size
byte 6      command class
byte 7      command id
bytes 8-87  arguments
byte 88     CRC — XOR of bytes 2..87
byte 89     reserved
```

Effects are command class `0x03`, command `0x0A`, with `arguments[0]` selecting the
effect (`0x00` off, `0x01` wave, `0x02` reactive, `0x03` breathing, `0x04` spectrum,
`0x05` custom frame, `0x06` static). Custom frames are pushed row by row with class
`0x03`, command `0x0B`: `[0xFF, row, start_col, stop_col, R, G, B, …]`.

Driver mode is class `0x00`, command `0x04`, argument `0x03`. Chroma boards ignore
effect writes until software takes LED ownership, so every apply begins with it.

### The key map

The 6 × 22 matrix positions were taken from OpenRGB's `BlackWidow Chroma TE` layout
(`RazerDevices.cpp`, PID `0x0209`) and cross-checked key-for-key against Razer's own
Chroma SDK enum `RzChromaSDKTypes.h`, where each `RZKEY` value is literally `0xRRCC`.
Both agree on all 87 keys. Column 0 is unused on the TKL (it holds M1–M5 on
full-size boards), and the spacebar is a single LED at `(5, 7)`.

## Limits

- The product ID is fixed at `0x0209` (`PRODUCT_ID` in `src/main.rs`). Other boards
  need their PID — and possibly a different effect dialect; the extended-matrix
  boards use a different command set.
- Firmware macros and key remapping live on the keyboard and are programmed from
  Windows Synapse. They are not reimplemented here yet: doing it properly on
  macOS means an event tap (Accessibility permission) for remapping and macro
  playback, which is a separate piece of work from the lighting path.
- Ripple and starlight are not in this board's firmware matrix effects.
- Effects are host-driven: unplugging the keyboard clears them. Reopening the app
  re-applies the saved state.

## License

MIT — see [LICENSE](LICENSE). Protocol knowledge from
[OpenRazer](https://github.com/openrazer/openrazer) (GPL-2.0) and
[OpenRGB](https://github.com/CalcProgrammer1/OpenRGB) (GPL-2.0); this project is an
independent implementation of the wire format, not a derivative of their code.
