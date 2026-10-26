/* Rig — console for the BlackWidow TE Chroma.
   The Rust service owns the USB protocol; this file is state, rendering, and HTTP. */

const U = 44;
const GAP = 6;
const STEP = U + GAP;

const k = (code, label, w = 1) => ({ code, label, w });
const gap = (w) => ({ gap: w });

const ROWS = [
  { keys: [k("Escape", "Esc"), gap(1),
    k("F1", "F1"), k("F2", "F2"), k("F3", "F3"), k("F4", "F4"), gap(0.5),
    k("F5", "F5"), k("F6", "F6"), k("F7", "F7"), k("F8", "F8"), gap(0.5),
    k("F9", "F9"), k("F10", "F10"), k("F11", "F11"), k("F12", "F12"), gap(0.5),
    k("PrintScreen", "PrtSc"), k("ScrollLock", "ScrLk"), k("Pause", "Pause")] },
  { keys: [k("Backquote", "`"), k("Digit1", "1"), k("Digit2", "2"), k("Digit3", "3"),
    k("Digit4", "4"), k("Digit5", "5"), k("Digit6", "6"), k("Digit7", "7"),
    k("Digit8", "8"), k("Digit9", "9"), k("Digit0", "0"), k("Minus", "-"),
    k("Equal", "="), k("Backspace", "Backspace", 2)] },
  { keys: [k("Tab", "Tab", 1.5), k("KeyQ", "Q"), k("KeyW", "W"), k("KeyE", "E"),
    k("KeyR", "R"), k("KeyT", "T"), k("KeyY", "Y"), k("KeyU", "U"), k("KeyI", "I"),
    k("KeyO", "O"), k("KeyP", "P"), k("BracketLeft", "["), k("BracketRight", "]"),
    k("Backslash", "\\", 1.5)] },
  { keys: [k("CapsLock", "Caps", 1.75), k("KeyA", "A"), k("KeyS", "S"), k("KeyD", "D"),
    k("KeyF", "F"), k("KeyG", "G"), k("KeyH", "H"), k("KeyJ", "J"), k("KeyK", "K"),
    k("KeyL", "L"), k("Semicolon", ";"), k("Quote", "'"), k("Enter", "Return", 2.25)] },
  { keys: [k("ShiftLeft", "Shift", 2.25), k("KeyZ", "Z"), k("KeyX", "X"), k("KeyC", "C"),
    k("KeyV", "V"), k("KeyB", "B"), k("KeyN", "N"), k("KeyM", "M"), k("Comma", ","),
    k("Period", "."), k("Slash", "/"), k("ShiftRight", "Shift", 2.75)] },
  { keys: [k("ControlLeft", "Ctrl", 1.25), k("MetaLeft", "Cmd", 1.25), k("AltLeft", "Opt", 1.25),
    k("Space", "", 6.25), k("AltRight", "Opt", 1.25), k("FnKey", "Fn", 1.25),
    k("ContextMenu", "Menu", 1.25), k("ControlRight", "Ctrl", 1.25)] },
];

const NAV = [
  { code: "Insert", label: "Ins", r: 1, c: 15.5 },
  { code: "Home", label: "Home", r: 1, c: 16.5 },
  { code: "PageUp", label: "PgUp", r: 1, c: 17.5 },
  { code: "Delete", label: "Del", r: 2, c: 15.5 },
  { code: "End", label: "End", r: 2, c: 16.5 },
  { code: "PageDown", label: "PgDn", r: 2, c: 17.5 },
  { code: "ArrowUp", label: "↑", r: 4, c: 16.5 },
  { code: "ArrowLeft", label: "←", r: 5, c: 15.5 },
  { code: "ArrowDown", label: "↓", r: 5, c: 16.5 },
  { code: "ArrowRight", label: "→", r: 5, c: 17.5 },
];

/* Physical key -> LED cell (row, column). From OpenRGB's BlackWidow Chroma TE
   layout, cross-checked against Razer's Chroma SDK RZKEY grid (0xRRCC). */
const KEY_CELLS = {
  Escape: [0, 1],
  F1: [0, 3], F2: [0, 4], F3: [0, 5], F4: [0, 6],
  F5: [0, 7], F6: [0, 8], F7: [0, 9], F8: [0, 10],
  F9: [0, 11], F10: [0, 12], F11: [0, 13], F12: [0, 14],
  PrintScreen: [0, 15], ScrollLock: [0, 16], Pause: [0, 17],
  Backquote: [1, 1],
  Digit1: [1, 2], Digit2: [1, 3], Digit3: [1, 4], Digit4: [1, 5],
  Digit5: [1, 6], Digit6: [1, 7], Digit7: [1, 8], Digit8: [1, 9],
  Digit9: [1, 10], Digit0: [1, 11], Minus: [1, 12], Equal: [1, 13],
  Backspace: [1, 14], Insert: [1, 15], Home: [1, 16], PageUp: [1, 17],
  Tab: [2, 1],
  KeyQ: [2, 2], KeyW: [2, 3], KeyE: [2, 4], KeyR: [2, 5],
  KeyT: [2, 6], KeyY: [2, 7], KeyU: [2, 8], KeyI: [2, 9],
  KeyO: [2, 10], KeyP: [2, 11], BracketLeft: [2, 12], BracketRight: [2, 13],
  Backslash: [2, 14], Delete: [2, 15], End: [2, 16], PageDown: [2, 17],
  CapsLock: [3, 1],
  KeyA: [3, 2], KeyS: [3, 3], KeyD: [3, 4], KeyF: [3, 5],
  KeyG: [3, 6], KeyH: [3, 7], KeyJ: [3, 8], KeyK: [3, 9],
  KeyL: [3, 10], Semicolon: [3, 11], Quote: [3, 12], Enter: [3, 14],
  ShiftLeft: [4, 1],
  KeyZ: [4, 3], KeyX: [4, 4], KeyC: [4, 5], KeyV: [4, 6],
  KeyB: [4, 7], KeyN: [4, 8], KeyM: [4, 9], Comma: [4, 10],
  Period: [4, 11], Slash: [4, 12], ShiftRight: [4, 14], ArrowUp: [4, 16],
  ControlLeft: [5, 1], MetaLeft: [5, 2], AltLeft: [5, 3], Space: [5, 7],
  AltRight: [5, 11], FnKey: [5, 12], ContextMenu: [5, 13], ControlRight: [5, 14],
  ArrowLeft: [5, 15], ArrowDown: [5, 16], ArrowRight: [5, 17],
};

const PALETTE = [
  "#ff3b30", "#ff9500", "#ffcc00", "#44d62c", "#00c7be",
  "#0a84ff", "#5e5ce6", "#bf5af2", "#ff2d55", "#f2f4ef",
];

const EFFECTS = [
  { id: "static", name: "Static", note: "one color, held still" },
  { id: "wave", name: "Wave", note: "sweeps across the board" },
  { id: "spectrum", name: "Spectrum", note: "full wheel, cycling" },
  { id: "breath", name: "Breathing", note: "fades up and down" },
  { id: "reactive", name: "Reactive", note: "lights up on keypress" },
  { id: "fire", name: "Fire", note: "flames licking upward" },
  { id: "starlight", name: "Starlight", note: "sparse twinkling points" },
  { id: "ripple", name: "Ripple", note: "rings travelling outward" },
  { id: "gradient", name: "Gradient", note: "two colors drifting" },
  { id: "marquee", name: "Marquee", note: "a band sweeping across" },
  { id: "comet", name: "Comet", note: "a head with a long tail" },
  { id: "type", name: "Typing", note: "ripples out from each keypress" },
  { id: "paint", name: "Custom frame", note: "your own colors, key by key" },
  { id: "off", name: "Off", note: "backlight dark" },
];

/* macOS virtual keycodes, indexed by KeyboardEvent.code. */
const KEYCODES = {
  KeyA: 0, KeyS: 1, KeyD: 2, KeyF: 3, KeyH: 4, KeyG: 5, KeyZ: 6, KeyX: 7, KeyC: 8, KeyV: 9,
  KeyB: 11, KeyQ: 12, KeyW: 13, KeyE: 14, KeyR: 15, KeyY: 16, KeyT: 17,
  Digit1: 18, Digit2: 19, Digit3: 20, Digit4: 21, Digit6: 22, Digit5: 23,
  Equal: 24, Digit9: 25, Digit7: 26, Minus: 27, Digit8: 28, Digit0: 29,
  BracketRight: 30, KeyO: 31, KeyU: 32, BracketLeft: 33, KeyI: 34, KeyP: 35,
  Enter: 36, KeyL: 37, KeyJ: 38, Quote: 39, KeyK: 40, Semicolon: 41,
  Backslash: 42, Comma: 43, Slash: 44, KeyN: 45, KeyM: 46, Period: 47,
  Tab: 48, Space: 49, Backquote: 50, Backspace: 51, Escape: 53,
  MetaRight: 54, MetaLeft: 55, ShiftLeft: 56, CapsLock: 57, AltLeft: 58,
  ControlLeft: 59, ShiftRight: 60, AltRight: 61, ControlRight: 62,
  F1: 122, F2: 120, F3: 99, F4: 118, F5: 96, F6: 97, F7: 98, F8: 100,
  F9: 101, F10: 109, F11: 103, F12: 111, Home: 115, PageUp: 116, Delete: 117,
  End: 119, PageDown: 121, ArrowLeft: 123, ArrowRight: 124, ArrowDown: 125,
  ArrowUp: 126,
};

const CODE_BY_KEY = Object.fromEntries(
  Object.entries(KEYCODES).map(([code, key]) => [key, code]),
);

function keyLabel(keycode) {
  const code = CODE_BY_KEY[keycode];
  if (!code) return `key ${keycode}`;
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code
    .replace("Arrow", "")
    .replace("Left", " left")
    .replace("Right", " right")
    .replace("Backspace", "delete")
    .replace("Backquote", "`")
    .replace("Semicolon", ";")
    .replace("Quote", "'")
    .replace("Comma", ",")
    .replace("Period", ".")
    .replace("Slash", "/")
    .replace("BracketLeft", "[")
    .replace("BracketRight", "]")
    .replace("Backslash", "\\")
    .replace("Minus", "-")
    .replace("Equal", "=");
}

const CAP_BASE = [27, 31, 33];
const SOFTWARE = new Set([
  "fire", "starlight", "ripple", "gradient", "marquee", "comet", "type",
]);
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

const dom = {
  keyboard: document.getElementById("keyboard"),
  studioKeyboard: document.getElementById("studio-keyboard"),
  ambient: document.getElementById("ambient"),
  studioAmbient: document.getElementById("studio-ambient"),
  chipLed: document.getElementById("chip-led"),
  chipStatus: document.getElementById("chip-status"),
  deviceSub: document.getElementById("device-sub"),
  readout: document.getElementById("readout"),
  alert: document.getElementById("alert"),
  effects: document.getElementById("effects"),
  wheel: document.getElementById("wheel"),
  hex: document.getElementById("hex"),
  palette: document.getElementById("palette"),
  brightness: document.getElementById("brightness"),
  brightnessOut: document.getElementById("brightness-out"),
  directionRow: document.getElementById("direction-row"),
  direction: document.getElementById("direction"),
  speedRow: document.getElementById("speed-row"),
  speed: document.getElementById("speed"),
  breathRow: document.getElementById("breath-row"),
  breath: document.getElementById("breath"),
  color2Row: document.getElementById("color2-row"),
  color2: document.getElementById("color2"),
  swatch2Fill: document.getElementById("swatch2-fill"),
  presetList: document.getElementById("presets"),
  presetsEmpty: document.getElementById("presets-empty"),
  saveForm: document.getElementById("save-form"),
  presetName: document.getElementById("preset-name"),
  driver: document.getElementById("driver"),
  gameMode: document.getElementById("game-mode"),
  macroMode: document.getElementById("macro-mode"),
  factFirmware: document.getElementById("fact-firmware"),
  factConnection: document.getElementById("fact-connection"),
  studioDot: document.getElementById("studio-dot"),
  studioHex: document.getElementById("studio-hex"),
  studioClear: document.getElementById("studio-clear"),
  studioApply: document.getElementById("studio-apply"),
  keysAlert: document.getElementById("keys-alert"),
  remapAdd: document.getElementById("remap-add"),
  remapHint: document.getElementById("remap-hint"),
  remaps: document.getElementById("remaps"),
  remapsEmpty: document.getElementById("remaps-empty"),
  macroName: document.getElementById("macro-name"),
  recordToggle: document.getElementById("record-toggle"),
  recordStatus: document.getElementById("record-status"),
  macros: document.getElementById("macros"),
  macrosEmpty: document.getElementById("macros-empty"),
  profileName: document.getElementById("profile-name"),
  profileSave: document.getElementById("profile-save"),
  profiles: document.getElementById("profiles"),
  profilesEmpty: document.getElementById("profiles-empty"),
  hyperBind: document.getElementById("hyper-bind"),
  hyperClear: document.getElementById("hyper-clear"),
  hyperState: document.getElementById("hyper-state"),
  remap2Add: document.getElementById("remap2-add"),
  remaps2: document.getElementById("remaps2"),
  remaps2Empty: document.getElementById("remaps2-empty"),
  gamma: document.getElementById("gamma"),
  gammaOut: document.getElementById("gamma-out"),
  gainR: document.getElementById("gain-r"),
  gainG: document.getElementById("gain-g"),
  gainB: document.getElementById("gain-b"),
  gainROut: document.getElementById("gain-r-out"),
  gainGOut: document.getElementById("gain-g-out"),
  gainBOut: document.getElementById("gain-b-out"),
};

let state = {
  effect: "static",
  color: "#ffffff",
  color2: "#0a84ff",
  brightness: 85,
  speed: 2,
  direction: 1,
  breath: "single",
  paint: {},
  driver: true,
  game_mode: false,
  macro_mode: false,
  gamma: 1,
  gain_r: 1,
  gain_g: 1,
  gain_b: 1,
};
let presets = [];
let firmware = null;
let tool = "brush";
let boards = [];
let applyTimer = null;
let frameTimer = null;
let frameColors = [];
let keysTimer = null;
let keysData = { trusted: false, recording: false, steps: 0, remaps: [], macros: [] };
let profilesData = { active: null, frontmost: null, profiles: [] };
let lastRevision = 0;
let lastApplyAt = 0;
let capture = null;
let pendingFrom = null;
let bindingMacro = null;
let captureLayer = 1;

/* ------------------------------------------------------------------ colors */

const hexToRgb = (hex) => {
  const v = hex.replace("#", "");
  return [
    parseInt(v.slice(0, 2), 16) || 0,
    parseInt(v.slice(2, 4), 16) || 0,
    parseInt(v.slice(4, 6), 16) || 0,
  ];
};

const rgbToHex = (rgb) =>
  "#" + rgb.map((x) => Math.round(Math.max(0, Math.min(255, x))).toString(16).padStart(2, "0")).join("");

const mix = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

function hsv(h, s, v) {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  let rgb;
  if (h < 60) rgb = [c, x, 0];
  else if (h < 120) rgb = [x, c, 0];
  else if (h < 180) rgb = [0, c, x];
  else if (h < 240) rgb = [0, x, c];
  else if (h < 300) rgb = [x, 0, c];
  else rgb = [c, 0, x];
  return rgb.map((value) => (value + m) * 255);
}

function rgbToHsv([r, g, b]) {
  const rn = r / 255, gn = g / 255, bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const delta = max - min;
  let hue = 0;
  if (delta !== 0) {
    if (max === rn) hue = 60 * (((gn - bn) / delta) % 6);
    else if (max === gn) hue = 60 * ((bn - rn) / delta + 2);
    else hue = 60 * ((rn - gn) / delta + 4);
  }
  if (hue < 0) hue += 360;
  return { h: hue, s: max === 0 ? 0 : delta / max, v: max };
}

/* ------------------------------------------------------------- color wheel */

const picker = { h: 20, s: 0.85, v: 1 };
let wheelContext = null;

const WHEEL = { size: 176, outer: 86, inner: 63, square: 82 };

function drawWheel() {
  const canvas = dom.wheel;
  if (!canvas) return;
  const dpr = window.devicePixelRatio || 1;
  if (canvas.width !== WHEEL.size * dpr) {
    canvas.width = WHEEL.size * dpr;
    canvas.height = WHEEL.size * dpr;
  }
  const ctx = canvas.getContext("2d");
  wheelContext = ctx;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, WHEEL.size, WHEEL.size);

  const c = WHEEL.size / 2;
  const step = 360 / 180;
  for (let a = 0; a < 360; a += step) {
    ctx.beginPath();
    ctx.arc(c, c, (WHEEL.outer + WHEEL.inner) / 2, ((a - 1.2) * Math.PI) / 180, ((a + step + 1.2) * Math.PI) / 180);
    ctx.strokeStyle = `hsl(${a} 100% 50%)`;
    ctx.lineWidth = WHEEL.outer - WHEEL.inner;
    ctx.stroke();
  }

  // saturation / value square
  const half = WHEEL.square / 2;
  const x0 = c - half;
  const y0 = c - half;
  ctx.fillStyle = `hsl(${picker.h} 100% 50%)`;
  ctx.fillRect(x0, y0, WHEEL.square, WHEEL.square);
  const white = ctx.createLinearGradient(x0, 0, x0 + WHEEL.square, 0);
  white.addColorStop(0, "#fff");
  white.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = white;
  ctx.fillRect(x0, y0, WHEEL.square, WHEEL.square);
  const black = ctx.createLinearGradient(0, y0, 0, y0 + WHEEL.square);
  black.addColorStop(0, "rgba(0,0,0,0)");
  black.addColorStop(1, "#000");
  ctx.fillStyle = black;
  ctx.fillRect(x0, y0, WHEEL.square, WHEEL.square);

  // markers
  const angle = (picker.h * Math.PI) / 180;
  ctx.beginPath();
  ctx.arc(c + Math.cos(angle) * ((WHEEL.outer + WHEEL.inner) / 2),
          c + Math.sin(angle) * ((WHEEL.outer + WHEEL.inner) / 2), 6, 0, Math.PI * 2);
  ctx.strokeStyle = "#fff";
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.beginPath();
  ctx.strokeStyle = "rgba(0,0,0,.55)";
  ctx.lineWidth = 1;
  ctx.stroke();
}

function wheelToColor(event) {
  const rect = dom.wheel.getBoundingClientRect();
  const x = ((event.clientX - rect.left) / rect.width) * WHEEL.size;
  const y = ((event.clientY - rect.top) / rect.height) * WHEEL.size;
  const c = WHEEL.size / 2;
  const dx = x - c;
  const dy = y - c;
  const distance = Math.hypot(dx, dy);

  if (distance > WHEEL.inner) {
    let angle = (Math.atan2(dy, dx) * 180) / Math.PI;
    if (angle < 0) angle += 360;
    picker.h = angle;
    drawWheel();
    commitPicker();
    return;
  }

  const half = WHEEL.square / 2;
  if (Math.abs(dx) <= half && Math.abs(dy) <= half) {
    picker.s = Math.min(1, Math.max(0, (dx + half) / WHEEL.square));
    picker.v = Math.min(1, Math.max(0, 1 - (dy + half) / WHEEL.square));
    drawWheel();
    commitPicker();
  }
}

function commitPicker() {
  const [r, g, b] = hsv(picker.h, picker.s, picker.v);
  state.color = rgbToHex([r, g, b]);
  reflectColor();
  invalidate();
  apply();
}

/* ---------------------------------------------------------------- keyboard */

function buildBoard(svg) {
  const ns = "http://www.w3.org/2000/svg";
  const frag = document.createDocumentFragment();
  const keys = [];

  const defs = document.createElementNS(ns, "defs");
  defs.innerHTML = `<linearGradient id="capGrad" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#31363a"/>
      <stop offset="0.55" stop-color="#24292c"/>
      <stop offset="1" stop-color="#181c1e"/>
    </linearGradient>`;
  frag.append(defs);

  const plate = document.createElementNS(ns, "rect");
  plate.setAttribute("class", "plate");
  plate.setAttribute("x", -14);
  plate.setAttribute("y", -14);
  plate.setAttribute("width", 18.5 * STEP - GAP + 28);
  plate.setAttribute("height", 5 * STEP + U + 28);
  plate.setAttribute("rx", 14);
  frag.append(plate);

  const draw = (code, label, x, y, width) => {
    const group = document.createElementNS(ns, "g");
    group.setAttribute("class", "key");
    group.dataset.code = code;
    const cell = KEY_CELLS[code];
    if (cell) group.dataset.cell = cell.join(",");

    const shadow = document.createElementNS(ns, "rect");
    shadow.setAttribute("class", "shadow");
    shadow.setAttribute("x", x);
    shadow.setAttribute("y", y + 3);
    shadow.setAttribute("width", width);
    shadow.setAttribute("height", U);
    shadow.setAttribute("rx", 7);

    const cap = document.createElementNS(ns, "rect");
    cap.setAttribute("class", "cap");
    cap.setAttribute("x", x);
    cap.setAttribute("y", y);
    cap.setAttribute("width", width);
    cap.setAttribute("height", U);
    cap.setAttribute("rx", 7);
    cap.setAttribute("fill", "url(#capGrad)");

    const under = document.createElementNS(ns, "rect");
    under.setAttribute("class", "under");
    under.setAttribute("x", x + 2);
    under.setAttribute("y", y + U - 5);
    under.setAttribute("width", width - 4);
    under.setAttribute("height", 3);
    under.setAttribute("rx", 1.5);
    under.style.opacity = "0";

    group.append(shadow, cap, under);

    if (label) {
      const text = document.createElementNS(ns, "text");
      text.setAttribute("class", "legend");
      text.setAttribute("x", x + width / 2);
      text.setAttribute("y", y + U / 2 + 0.5);
      text.textContent = label;
      group.append(text);
    }

    frag.append(group);
    keys.push({ code, cap, under, group, x, width });
  };

  ROWS.forEach((row, index) => {
    let cursor = 0;
    for (const item of row.keys) {
      if (item.gap) {
        cursor += item.gap;
        continue;
      }
      draw(item.code, item.label, cursor * STEP, index * STEP, item.w * U + (item.w - 1) * GAP);
      cursor += item.w;
    }
  });

  for (const nav of NAV) draw(nav.code, nav.label, nav.c * STEP, nav.r * STEP, U);

  svg.replaceChildren(frag);
  svg.setAttribute("viewBox", `0 0 ${18.5 * STEP - GAP + 4} ${5 * STEP + U + 12}`);
  return { keys, plate };
}

/// Lighting preview: the caps stay black, the light shows as a strip bleeding
/// out from under each cap plus the legends.
function paintKey(entry, color, intensity) {
  entry.under.style.fill = rgbToHex(color);
  entry.under.style.opacity = String(0.92 * intensity);
  entry.group.classList.toggle("lit", intensity > 0.5);
}

/// Studio: this surface is for editing, so painted keys show their colour.
function paintKeyEditing(entry, color, painted) {
  entry.cap.style.fill = painted ? rgbToHex(mix(CAP_BASE, color, 0.72)) : "url(#capGrad)";
  entry.under.style.fill = rgbToHex(color);
  entry.under.style.opacity = painted ? "0.9" : "0";
  entry.group.classList.toggle("lit", painted);
}

/* ---------------------------------------------------------------- preview */

function keyColors(now) {
  const colors = new Map();

  for (const entry of boards[0].keys) {
    const cell = entry.group.dataset.cell;
    let color = CAP_BASE;
    let intensity = 0;

    if (state.effect === "off") {
      colors.set(entry.code, { color, intensity });
      continue;
    }

    if (state.effect === "paint") {
      const hex = cell ? state.paint[cell] : null;
      colors.set(entry.code, hex
        ? { color: hexToRgb(hex), intensity: 1 }
        : { color: CAP_BASE, intensity: 0 });
      continue;
    }

    // Everything else is drawn by the service; show exactly what it sent.
    if (cell && frameColors.length === 132) {
      const [row, col] = cell.split(",").map(Number);
      color = hexToRgb(frameColors[row * 22 + col]);
      intensity = 1;
    } else {
      color = hexToRgb(state.color);
      intensity = 0.35;
    }

    // The board dims via its own brightness cap; mirror that here.
    colors.set(entry.code, {
      color,
      intensity: intensity * (0.25 + 0.75 * (state.brightness / 100)),
    });
  }

  return colors;
}

function render(now = 0) {
  if (currentView === "lighting") {
    const colors = keyColors(now);
    for (const entry of boards[0].keys) {
      const value = colors.get(entry.code) || { color: CAP_BASE, intensity: 0 };
      paintKey(entry, value.color, value.intensity);
    }
    const base = hexToRgb(state.color);
    const spill = state.effect === "off" ? 0 : 0.12 + (state.brightness / 100) * 0.3;
    boards[0].plate.style.fill = rgbToHex(mix([13, 16, 17], base, spill * 0.5));
    dom.ambient.style.opacity =
      String(state.effect === "off" ? 0.05 : 0.2 + (state.brightness / 100) * 0.5);
  } else if (currentView === "studio") {
    for (const entry of boards[1].keys) {
      const hex = state.paint[entry.group.dataset.cell];
      paintKeyEditing(entry, hex ? hexToRgb(hex) : CAP_BASE, Boolean(hex));
    }
    const base = hexToRgb(state.color);
    boards[1].plate.style.fill = rgbToHex(mix([13, 16, 17], base, 0.16));
    dom.studioAmbient.style.opacity = String(0.2 + (state.brightness / 100) * 0.5);
  }

  dom.readout.textContent = `${labelFor(state.effect)} · ${state.color.toUpperCase()}`;
  dom.studioDot.style.background = state.color;
  dom.studioHex.textContent = state.color.toUpperCase();
}

function labelFor(id) {
  return (EFFECTS.find((effect) => effect.id === id) || EFFECTS[0]).name;
}

const ANIMATED = new Set(["spectrum", "wave", "breath", "reactive"]);
let currentView = "lighting";
let frameHandle = null;
let dirty = true;

function wantsMotion() {
  return ANIMATED.has(state.effect)
    && !reducedMotion.matches
    && !document.hidden
    && currentView === "lighting";
}

function tick(now) {
  frameHandle = null;
  if (dirty || wantsMotion()) {
    render(now);
    dirty = false;
  }
  if (wantsMotion()) frameHandle = requestAnimationFrame(tick);
}

/// Ask for one paint on the next frame instead of painting synchronously in
/// every input handler.
function invalidate() {
  dirty = true;
  if (frameHandle === null) frameHandle = requestAnimationFrame(tick);
}

/* -------------------------------------------------------------------- http */

const payload = () => ({
  effect: state.effect,
  color: state.color,
  color2: state.color2,
  brightness: state.brightness,
  speed: state.speed,
  direction: state.direction,
  breath: state.breath,
  paint: state.paint,
  driver: state.driver,
  game_mode: state.game_mode,
  macro_mode: state.macro_mode,
  gamma: state.gamma,
  gain_r: state.gain_r,
  gain_g: state.gain_g,
  gain_b: state.gain_b,
});

async function request(url, options = {}) {
  const response = await fetch(url, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!response.ok) throw new Error(String(response.status));
  return response.json();
}

async function pollState() {
  if (document.hidden) return;
  try {
    const data = await request("/api/state");
    if (typeof data.revision !== "number" || data.revision === lastRevision) return;
    lastRevision = data.revision;
    // Do not stomp on an edit that is still being sent.
    if (!data.state || Date.now() - lastApplyAt < 1200) return;
    state = { ...state, ...data.state };
    syncAll();
    invalidate();
  } catch {
    /* service restarting */
  }
}

function syncPolling() {
  const wanted = currentView === "lighting" && !document.hidden
    && state.effect !== "off" && state.effect !== "paint";
  if (wanted && frameTimer === null) {
    fetchFrame();
    frameTimer = setInterval(fetchFrame, 80);
  } else if (!wanted && frameTimer !== null) {
    clearInterval(frameTimer);
    frameTimer = null;
  }
}

async function fetchFrame() {
  try {
    const colors = await request("/api/frame");
    if (Array.isArray(colors) && colors.length === 132) {
      frameColors = colors;
      invalidate();
    }
  } catch {
    /* service restarting; the next tick retries */
  }
}

function apply({ immediate = false } = {}) {
  lastApplyAt = Date.now();
  clearTimeout(applyTimer);
  const send = () => request("/api/apply", { method: "POST", body: JSON.stringify(payload()) })
    .then(updateFromServer)
    .catch(() => setStatus(false, "Service not responding"));
  if (immediate) send();
  else applyTimer = setTimeout(send, 90);
}

function updateFromServer(data) {
  if (!data) return;
  if (typeof data.revision === "number") lastRevision = data.revision;
  if (data.firmware) firmware = data.firmware;
  if (data.presets) presets = data.presets;
  setStatus(data.connected, data.firmware, data.error);
  if (data.error) showAlert(data.error);
  else hideAlert();
  renderPresets();
}

function setStatus(connected, version, error) {
  dom.chipLed.dataset.on = String(Boolean(connected));
  dom.chipStatus.textContent = connected
    ? "Connected"
    : error ? "Not reachable" : "Not connected";
  dom.deviceSub.textContent = `Razer · 1532:0209 · ${
    version ? `firmware ${version}` : "firmware unknown"}`;
  dom.factFirmware.textContent = version || "—";
  dom.factConnection.textContent = connected ? "Connected" : "Disconnected";
}

function showAlert(message) {
  dom.alert.hidden = false;
  dom.alert.innerHTML = `<strong>The keyboard didn’t answer.</strong> ${message}`;
}

const hideAlert = () => { dom.alert.hidden = true; };

/* ------------------------------------------------------------------- views */

function showView(name) {
  for (const tab of document.querySelectorAll(".tab")) {
    tab.setAttribute("aria-selected", String(tab.dataset.tab === name));
  }
  for (const view of document.querySelectorAll(".view")) {
    view.hidden = view.dataset.view !== name;
  }
  currentView = name;
  invalidate();
  syncPolling();
  syncKeysPolling();
}

/* ---------------------------------------------------------------- controls */

function buildEffects() {
  const frag = document.createDocumentFragment();
  for (const effect of EFFECTS) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "effect";
    button.dataset.effect = effect.id;
    button.setAttribute("role", "radio");
    button.setAttribute("aria-checked", "false");
    button.innerHTML = `<span class="effect-name">${effect.name}</span>
      <span class="effect-note">${effect.note}</span>`;
    frag.append(button);
  }
  dom.effects.replaceChildren(frag);
}

function syncEffects() {
  for (const button of dom.effects.querySelectorAll(".effect")) {
    button.setAttribute("aria-checked", String(button.dataset.effect === state.effect));
  }
  const software = SOFTWARE.has(state.effect);
  dom.directionRow.hidden = state.effect !== "wave";
  dom.speedRow.hidden = !(state.effect === "reactive" || software);
  dom.breathRow.hidden = state.effect !== "breath";
  dom.color2Row.hidden = !(
    (state.effect === "breath" && state.breath === "dual") || state.effect === "gradient"
  );

  for (const button of dom.direction.querySelectorAll("button")) {
    button.setAttribute("aria-pressed", String(Number(button.dataset.direction) === state.direction));
  }
  for (const button of dom.speed.querySelectorAll("button")) {
    button.setAttribute("aria-pressed", String(Number(button.dataset.speed) === state.speed));
  }
  for (const button of dom.breath.querySelectorAll("button")) {
    button.setAttribute("aria-pressed", String(button.dataset.breath === state.breath));
  }
  syncPolling();
}

function reflectColor() {
  dom.hex.value = state.color.replace("#", "").toUpperCase();
  document.documentElement.style.setProperty("--light", state.color);
  dom.color2.value = state.color2;
  dom.swatch2Fill.style.background = state.color2;
  for (const dot of dom.palette.querySelectorAll(".dot")) {
    dot.setAttribute("aria-pressed",
      String(dot.dataset.color.toLowerCase() === state.color.toLowerCase()));
  }
}

function syncColor() {
  const { h, s, v } = rgbToHsv(hexToRgb(state.color));
  picker.h = h;
  picker.s = s;
  picker.v = Math.max(v, 0.02);
  drawWheel();
  reflectColor();
}

function syncCalibration() {
  dom.gamma.value = String(state.gamma);
  dom.gammaOut.value = Number(state.gamma).toFixed(2);
  dom.gainR.value = String(Math.round(state.gain_r * 100));
  dom.gainG.value = String(Math.round(state.gain_g * 100));
  dom.gainB.value = String(Math.round(state.gain_b * 100));
  dom.gainROut.value = String(Math.round(state.gain_r * 100));
  dom.gainGOut.value = String(Math.round(state.gain_g * 100));
  dom.gainBOut.value = String(Math.round(state.gain_b * 100));
}

function syncBrightness() {
  dom.brightness.value = String(state.brightness);
  dom.brightnessOut.value = String(state.brightness);
  dom.brightness.style.setProperty("--fill", `${state.brightness}%`);
}

function syncSwitches() {
  dom.driver.setAttribute("aria-checked", String(state.driver));
  dom.gameMode.setAttribute("aria-checked", String(state.game_mode));
  dom.macroMode.setAttribute("aria-checked", String(state.macro_mode));
}

function renderPresets() {
  const frag = document.createDocumentFragment();
  for (const preset of presets) {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "preset-card";
    card.dataset.id = String(preset.id);
    card.innerHTML = `
      <span class="preset-preview" style="--c:${preset.state.color}"><i></i>
        <span>${preset.state.color.toUpperCase()}</span></span>
      <span class="preset-meta">
        <span class="preset-name"></span>
        <span class="preset-effect">${escapeHtml(labelFor(preset.state.effect))}</span>
      </span>
      <span class="preset-remove" role="button" tabindex="0" aria-label="Delete preset">×</span>`;
    card.querySelector(".preset-name").textContent = preset.name;
    frag.append(card);
  }
  dom.presetList.replaceChildren(frag);
  dom.presetsEmpty.hidden = presets.length > 0;
}

const escapeHtml = (value) => value.replace(/[&<>"']/g, (c) => ({
  "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
}[c]));

function syncAll() {
  syncEffects();
  syncColor();
  syncBrightness();
  syncCalibration();
  syncSwitches();
}

/* -------------------------------------------------------------- interaction */

function bind() {
  document.querySelector(".tabs").addEventListener("click", (event) => {
    const tab = event.target.closest(".tab");
    if (tab) showView(tab.dataset.tab);
  });

  dom.effects.addEventListener("click", (event) => {
    const button = event.target.closest(".effect");
    if (!button) return;
    state.effect = button.dataset.effect;
    syncEffects();
    invalidate();
    apply();
  });

  dom.effects.addEventListener("keydown", (event) => {
    if (!["ArrowDown", "ArrowUp"].includes(event.key)) return;
    const buttons = [...dom.effects.querySelectorAll(".effect")];
    const index = buttons.findIndex((b) => b.getAttribute("aria-checked") === "true");
    const next = (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    event.preventDefault();
    buttons[next].focus();
    state.effect = buttons[next].dataset.effect;
    syncEffects();
    invalidate();
    apply();
  });

  for (const [group, key] of [[dom.direction, "direction"], [dom.speed, "speed"], [dom.breath, "breath"]]) {
    group.addEventListener("click", (event) => {
      const button = event.target.closest("button");
      if (!button) return;
      state[key] = key === "breath" ? button.dataset.breath : Number(button.dataset[key]);
      syncEffects();
      invalidate();
      apply();
    });
  }

  let dragging = false;
  dom.wheel.addEventListener("pointerdown", (event) => {
    dragging = true;
    dom.wheel.setPointerCapture(event.pointerId);
    wheelToColor(event);
  });
  dom.wheel.addEventListener("pointermove", (event) => {
    if (dragging) wheelToColor(event);
  });
  const release = (event) => {
    dragging = false;
    try { dom.wheel.releasePointerCapture(event.pointerId); } catch { /* ignore */ }
  };
  dom.wheel.addEventListener("pointerup", release);
  dom.wheel.addEventListener("pointercancel", release);

  dom.color2.addEventListener("input", () => {
    state.color2 = dom.color2.value;
    syncColor();
    invalidate();
    apply();
  });

  dom.hex.addEventListener("change", () => {
    const value = dom.hex.value.trim().replace("#", "");
    if (/^[0-9a-f]{6}$/i.test(value)) {
      state.color = `#${value.toLowerCase()}`;
      syncColor();
      invalidate();
      apply({ immediate: true });
    } else {
      syncColor();
    }
  });

  dom.hex.addEventListener("keydown", (event) => {
    if (event.key === "Enter") dom.hex.blur();
  });

  const bindCalibration = (input, output, key, scale) => {
    input.addEventListener("input", () => {
      state[key] = Number(input.value) * scale;
      output.value = key === "gamma"
        ? state[key].toFixed(2)
        : String(Math.round(Number(input.value)));
      invalidate();
      apply();
    });
  };

  bindCalibration(dom.gamma, dom.gammaOut, "gamma", 1);
  bindCalibration(dom.gainR, dom.gainROut, "gain_r", 0.01);
  bindCalibration(dom.gainG, dom.gainGOut, "gain_g", 0.01);
  bindCalibration(dom.gainB, dom.gainBOut, "gain_b", 0.01);

  dom.brightness.addEventListener("input", () => {
    state.brightness = Number(dom.brightness.value);
    syncBrightness();
    invalidate();
    apply();
  });

  for (const [element, key] of [
    [dom.driver, "driver"],
    [dom.gameMode, "game_mode"],
    [dom.macroMode, "macro_mode"],
  ]) {
    element.addEventListener("click", () => {
      state[key] = !state[key];
      syncSwitches();
      invalidate();
      apply({ immediate: true });
    });
  }

  dom.studioApply.addEventListener("click", () => {
    state.effect = "paint";
    syncEffects();
    invalidate();
    apply({ immediate: true });
  });

  dom.studioClear.addEventListener("click", () => {
    state.paint = {};
    invalidate();
  });

  for (const button of document.querySelectorAll(".tool[data-tool]")) {
    button.addEventListener("click", () => {
      tool = button.dataset.tool;
      for (const other of document.querySelectorAll(".tool[data-tool]")) {
        other.setAttribute("aria-pressed", String(other === button));
      }
      if (tool === "fill") {
        state.paint = {};
        for (const entry of boards[0].keys) {
          const cell = entry.group.dataset.cell;
          if (cell) state.paint[cell] = state.color;
        }
        invalidate();
      }
    });
  }

  dom.studioKeyboard.addEventListener("click", (event) => {
    const group = event.target.closest(".key");
    if (!group || !group.dataset.cell) return;
    const cell = group.dataset.cell;
    if (tool === "brush") state.paint[cell] = state.color;
    else if (tool === "eraser") delete state.paint[cell];
    else return;
    invalidate();
  });

  const beginCapture = (layer, message, button) => {
    if (capture) {
      cancelCapture();
      return;
    }
    capture = "from";
    captureLayer = layer;
    pendingFrom = null;
    if (button) button.textContent = "Cancel";
    dom.remapHint.textContent = message;
  };

  dom.remapAdd.addEventListener("click", () => {
    if (capture) {
      cancelCapture();
      return;
    }
    beginCapture(1, "Press the key you want to change…", dom.remapAdd);
  });

  dom.remap2Add.addEventListener("click", () => {
    if (capture) {
      cancelCapture();
      return;
    }
    beginCapture(2, "Press the layer 2 key to change…", dom.remap2Add);
  });

  dom.hyperBind.addEventListener("click", () => {
    capture = "hyper";
    dom.remapHint.textContent = "Hold-to-shift key: press it now…";
  });

  dom.hyperClear.addEventListener("click", () => {
    request("/api/keys/hypershift", {
      method: "POST",
      body: JSON.stringify({ key: null }),
    }).then((data) => {
      keysData = data;
      renderKeys();
    });
  });

  document.addEventListener("keydown", (event) => {
    if (!capture) return;
    if (event.key === "Escape") {
      cancelCapture();
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const keycode = KEYCODES[event.code];
    if (keycode === undefined) {
      dom.remapHint.textContent = "That key has no mapping yet — try another.";
      return;
    }
    if (capture === "hyper") {
      request("/api/keys/hypershift", {
        method: "POST",
        body: JSON.stringify({ key: keycode }),
      }).then((data) => {
        keysData = data;
        renderKeys();
        cancelCapture();
      });
      return;
    }

    if (capture === "bind") {
      request("/api/keys/macro/bind", {
        method: "POST",
        body: JSON.stringify({ id: bindingMacro, key: keycode }),
      }).then((data) => {
        keysData = data;
        renderKeys();
        cancelCapture();
        dom.recordStatus.hidden = true;
      });
      return;
    }

    if (capture === "from") {
      pendingFrom = keycode;
      capture = "to";
      dom.remapHint.textContent =
        `Now press what ${keyLabel(keycode)} should act as.`;
      return;
    }
    request("/api/keys/remap", {
      method: "POST",
      body: JSON.stringify({ from: pendingFrom, to: keycode, layer: captureLayer }),
    }).then((data) => {
      keysData = data;
      renderKeys();
      cancelCapture();
    });
  }, true);

  dom.recordToggle.addEventListener("click", () => {
    if (keysData.recording) {
      request("/api/keys/record/stop", {
        method: "POST",
        body: JSON.stringify({ name: dom.macroName.value.trim() }),
      }).then((data) => {
        if (data.error) dom.recordStatus.textContent = data.error;
        else dom.macroName.value = "";
        return request("/api/keys");
      }).then((data) => {
        keysData = data;
        renderKeys();
      });
    } else {
      request("/api/keys/record/start", { method: "POST" }).then((data) => {
        keysData = data;
        renderKeys();
      });
    }
  });

  dom.profileSave.addEventListener("click", () => {
    const name = dom.profileName.value.trim();
    if (!name) return;
    request("/api/profiles", { method: "POST", body: JSON.stringify({ name }) })
      .then((data) => {
        profilesData = data;
        dom.profileName.value = "";
        renderProfiles();
      });
  });

  dom.profiles.addEventListener("click", (event) => {
    const activate = event.target.closest("[data-activate]");
    const bind = event.target.closest("[data-bind-app]");
    const remove = event.target.closest("[data-remove-profile]");
    const post = (url, body) => request(url, { method: "POST", body: JSON.stringify(body) })
      .then((data) => {
        profilesData = data;
        renderProfiles();
        return pollState();
      });
    if (activate) post("/api/profiles/activate", { id: Number(activate.dataset.activate) });
    else if (bind) post("/api/profiles/bind", { id: Number(bind.dataset.bindApp) });
    else if (remove) post("/api/profiles/delete", { id: Number(remove.dataset.removeProfile) });
  });

  const removeRemap = (from, layer) => request("/api/keys/remap/delete", {
    method: "POST",
    body: JSON.stringify({ from, to: 0, layer }),
  }).then((data) => {
    keysData = data;
    renderKeys();
  });

  dom.remaps.addEventListener("click", (event) => {
    const button = event.target.closest("[data-remove]");
    if (button) removeRemap(Number(button.dataset.remove), 1);
  });

  dom.remaps2.addEventListener("click", (event) => {
    const button = event.target.closest("[data-remove]");
    if (button) removeRemap(Number(button.dataset.remove), 2);
  });

  dom.macros.addEventListener("click", (event) => {
    const bind = event.target.closest("[data-bind]");
    const play = event.target.closest("[data-play]");
    const remove = event.target.closest("[data-delete]");
    if (bind) {
      capture = "bind";
      bindingMacro = Number(bind.dataset.bind);
      dom.recordStatus.hidden = false;
      dom.recordStatus.textContent = "Press the key that should run this macro…";
      return;
    }
    if (play) {
      request("/api/keys/macro/play", {
        method: "POST",
        body: JSON.stringify({ id: Number(play.dataset.play) }),
      }).then((data) => {
        keysData = data;
        renderKeys();
      });
    } else if (remove) {
      request("/api/keys/macro/delete", {
        method: "POST",
        body: JSON.stringify({ id: Number(remove.dataset.delete) }),
      }).then((data) => {
        keysData = data;
        renderKeys();
      });
    }
  });

  dom.presetList.addEventListener("click", (event) => {
    const card = event.target.closest(".preset-card");
    if (!card) return;
    if (event.target.closest(".preset-remove")) {
      request("/api/presets/delete", {
        method: "POST",
        body: JSON.stringify({ id: Number(card.dataset.id) }),
      }).then(updateFromServer);
      return;
    }
    const preset = presets.find((item) => item.id === Number(card.dataset.id));
    if (!preset) return;
    state = { ...state, ...preset.state, paint: { ...preset.state.paint } };
    syncAll();
    invalidate();
    showView("lighting");
    apply({ immediate: true });
  });

  dom.presetList.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    const card = event.target.closest(".preset-card");
    if (!card) return;
    event.preventDefault();
    if (event.target.closest(".preset-remove")) {
      request("/api/presets/delete", {
        method: "POST",
        body: JSON.stringify({ id: Number(card.dataset.id) }),
      }).then(updateFromServer);
    } else {
      card.click();
    }
  });

  dom.saveForm.addEventListener("submit", (event) => {
    event.preventDefault();
    const name = dom.presetName.value.trim();
    if (!name) return;
    request("/api/presets", { method: "POST", body: JSON.stringify({ name, state: payload() }) })
      .then((data) => {
        dom.presetName.value = "";
        updateFromServer(data);
      });
  });

  reducedMotion.addEventListener("change", () => render());
}

function buildPalette() {
  const frag = document.createDocumentFragment();
  for (const color of PALETTE) {
    const dot = document.createElement("button");
    dot.type = "button";
    dot.className = "dot";
    dot.dataset.color = color;
    dot.style.background = color;
    dot.setAttribute("aria-label", `Use ${color}`);
    dot.setAttribute("aria-pressed", "false");
    dot.addEventListener("click", () => {
      state.color = color;
      syncColor();
      invalidate();
      apply();
    });
    frag.append(dot);
  }
  dom.palette.replaceChildren(frag);
}

/* --------------------------------------------------------------------- keys */

function cancelCapture() {
  capture = null;
  pendingFrom = null;
  bindingMacro = null;
  dom.remapAdd.textContent = "Add a remap";
  dom.remap2Add.textContent = "Add a layer 2 remap";
  dom.remapHint.textContent =
    "Press the key you want to change, then press what it should act as.";
}

function syncKeysPolling() {
  const wanted = currentView === "keys" && !document.hidden;
  if (wanted && keysTimer === null) {
    fetchKeys();
    keysTimer = setInterval(fetchKeys, 800);
  } else if (!wanted && keysTimer !== null) {
    clearInterval(keysTimer);
    keysTimer = null;
  }
}

async function fetchKeys() {
  try {
    keysData = await request("/api/keys");
    renderKeys();
  } catch {
    /* service restarting */
  }
}

function renderKeys() {
  dom.keysAlert.hidden = keysData.trusted;
  if (!keysData.trusted) {
    dom.keysAlert.innerHTML =
      "<strong>Accessibility permission is needed.</strong> Remaps and macros only "
      + "work once this app may control the computer: System Settings → Privacy & "
      + "Security → Accessibility.";
  }

  const remaps = document.createDocumentFragment();
  for (const [from, to] of keysData.remaps) {
    const row = document.createElement("div");
    row.className = "row-item";
    row.innerHTML = `
      <span class="row-main">
        <span class="keycap">${keyLabel(from)}</span>
        <span class="row-arrow">→</span>
        <span class="keycap">${keyLabel(to)}</span>
      </span>
      <button class="icon-button" data-remove="${from}" aria-label="Remove remap">×</button>`;
    remaps.append(row);
  }
  dom.remaps.replaceChildren(remaps);
  dom.remapsEmpty.hidden = keysData.remaps.length > 0;

  const layer2 = document.createDocumentFragment();
  for (const [from, to] of keysData.remaps2 || []) {
    const row = document.createElement("div");
    row.className = "row-item";
    row.innerHTML = `
      <span class="row-main">
        <span class="keycap">${keyLabel(from)}</span>
        <span class="row-arrow">→</span>
        <span class="keycap">${keyLabel(to)}</span>
      </span>
      <button class="icon-button" data-remove="${from}" aria-label="Remove remap">×</button>`;
    layer2.append(row);
  }
  dom.remaps2.replaceChildren(layer2);
  dom.remaps2Empty.hidden = (keysData.remaps2 || []).length > 0;
  dom.hyperState.textContent = keysData.hypershift == null
    ? "Not set"
    : `Hold ${keyLabel(keysData.hypershift)}`;

  const macros = document.createDocumentFragment();
  for (const macro of keysData.macros) {
    const row = document.createElement("div");
    row.className = "row-item";
    const bound = macro.trigger == null ? "" : `<span class="keycap">${keyLabel(macro.trigger)}</span>`;
    row.innerHTML = `
      <span class="row-main">
        <span class="row-name">${escapeHtml(macro.name)}</span>
        <span class="row-meta">${macro.steps.length} events</span>
        ${bound}
      </span>
      <span class="row-main">
        <button class="ghost" data-bind="${macro.id}">${macro.trigger == null ? "Bind" : "Rebind"}</button>
        <button class="ghost" data-play="${macro.id}">Play</button>
        <button class="icon-button" data-delete="${macro.id}" aria-label="Delete macro">×</button>
      </span>`;
    macros.append(row);
  }
  dom.macros.replaceChildren(macros);
  dom.macrosEmpty.hidden = keysData.macros.length > 0;

  dom.recordToggle.textContent = keysData.recording ? "Stop" : "Record";
  dom.recordToggle.classList.toggle("recording", keysData.recording);
  dom.recordStatus.hidden = !keysData.recording;
  if (keysData.recording) {
    dom.recordStatus.textContent =
      `Recording — ${keysData.steps} key events captured so far.`;
  }
}

/* ----------------------------------------------------------------- profiles */

async function fetchProfiles() {
  try {
    profilesData = await request("/api/profiles");
    renderProfiles();
  } catch {
    /* service restarting */
  }
}

function renderProfiles() {
  const frag = document.createDocumentFragment();
  for (const profile of profilesData.profiles) {
    const row = document.createElement("div");
    row.className = "row-item";
    const bound = profile.bundle
      ? `<span class="row-meta">${escapeHtml(profile.bundle.split(".").pop())}</span>`
      : `<span class="row-meta">any app</span>`;
    const active = profilesData.active === profile.id
      ? `<span class="keycap">active</span>`
      : `<button class="ghost" data-activate="${profile.id}">Use</button>`;
    row.innerHTML = `
      <span class="row-main">
        <span class="row-name">${escapeHtml(profile.name)}</span>
        ${bound}
        ${active}
      </span>
      <span class="row-main">
        <button class="ghost" data-bind-app="${profile.id}">Bind to current app</button>
        <button class="icon-button" data-remove-profile="${profile.id}" aria-label="Delete profile">×</button>
      </span>`;
    frag.append(row);
  }
  dom.profiles.replaceChildren(frag);
  dom.profilesEmpty.hidden = profilesData.profiles.length > 0;
}

/* -------------------------------------------------------------------- boot */

async function boot() {
  buildEffects();
  buildPalette();
  boards = [buildBoard(dom.keyboard), buildBoard(dom.studioKeyboard)];
  bind();
  syncAll();
  renderPresets();
  fetchProfiles();
  invalidate();

  try {
    const data = await request("/api/state");
    if (data.state) state = { ...state, ...data.state };
    presets = data.presets || [];
    firmware = data.firmware;
    syncAll();
    setStatus(data.connected, data.firmware, data.error);
    if (data.error) showAlert(data.error);
    renderPresets();
  } catch {
    setStatus(false);
    showAlert("The local service isn’t running. Start `rig` and reload this page.");
  }

  invalidate();
}

document.addEventListener("visibilitychange", () => {
  invalidate();
  syncPolling();
  syncKeysPolling();
});

setInterval(pollState, 2000);

boot();
