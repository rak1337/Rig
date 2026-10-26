/* Rig — lighting over WebHID, no service behind it.
   Speaks the same 90-byte Razer reports as the native build. */

const U = 44, GAP = 6, STEP = U + GAP, COLS = 22, ROWS_N = 6;
const BOARD_W = 18.5 * STEP - GAP;
const BOARD_H = 5 * STEP + U;
const CASE = { pad: 26, top: 18, lip: 16 };

const k = (code, label, w = 1, shift = null) => ({ code, label, w, shift });
const gap = (w) => ({ gap: w });

const ROWS = [
  { keys: [k("Escape", "Esc"), gap(1),
    k("F1", "F1"), k("F2", "F2"), k("F3", "F3"), k("F4", "F4"), gap(0.5),
    k("F5", "F5"), k("F6", "F6"), k("F7", "F7"), k("F8", "F8"), gap(0.5),
    k("F9", "F9"), k("F10", "F10"), k("F11", "F11"), k("F12", "F12"), gap(0.5),
    k("PrintScreen", "PrtSc"), k("ScrollLock", "ScrLk"), k("Pause", "Pause")] },
  { keys: [k("Backquote", "`", 1, "~"), k("Digit1", "1", 1, "!"), k("Digit2", "2", 1, "@"),
    k("Digit3", "3", 1, "#"), k("Digit4", "4", 1, "$"), k("Digit5", "5", 1, "%"),
    k("Digit6", "6", 1, "^"), k("Digit7", "7", 1, "&"), k("Digit8", "8", 1, "*"),
    k("Digit9", "9", 1, "("), k("Digit0", "0", 1, ")"), k("Minus", "-", 1, "_"),
    k("Equal", "=", 1, "+"), k("Backspace", "Backspace", 2)] },
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
  { code: "Insert", label: "Ins", r: 1, c: 15.5 }, { code: "Home", label: "Home", r: 1, c: 16.5 },
  { code: "PageUp", label: "PgUp", r: 1, c: 17.5 }, { code: "Delete", label: "Del", r: 2, c: 15.5 },
  { code: "End", label: "End", r: 2, c: 16.5 }, { code: "PageDown", label: "PgDn", r: 2, c: 17.5 },
  { code: "ArrowUp", label: "↑", r: 4, c: 16.5 }, { code: "ArrowLeft", label: "←", r: 5, c: 15.5 },
  { code: "ArrowDown", label: "↓", r: 5, c: 16.5 }, { code: "ArrowRight", label: "→", r: 5, c: 17.5 },
];

const KEY_CELLS = {
  Escape: [0, 1], F1: [0, 3], F2: [0, 4], F3: [0, 5], F4: [0, 6], F5: [0, 7], F6: [0, 8],
  F7: [0, 9], F8: [0, 10], F9: [0, 11], F10: [0, 12], F11: [0, 13], F12: [0, 14],
  PrintScreen: [0, 15], ScrollLock: [0, 16], Pause: [0, 17], Backquote: [1, 1],
  Digit1: [1, 2], Digit2: [1, 3], Digit3: [1, 4], Digit4: [1, 5], Digit5: [1, 6],
  Digit6: [1, 7], Digit7: [1, 8], Digit8: [1, 9], Digit9: [1, 10], Digit0: [1, 11],
  Minus: [1, 12], Equal: [1, 13], Backspace: [1, 14], Insert: [1, 15], Home: [1, 16],
  PageUp: [1, 17], Tab: [2, 1], KeyQ: [2, 2], KeyW: [2, 3], KeyE: [2, 4], KeyR: [2, 5],
  KeyT: [2, 6], KeyY: [2, 7], KeyU: [2, 8], KeyI: [2, 9], KeyO: [2, 10], KeyP: [2, 11],
  BracketLeft: [2, 12], BracketRight: [2, 13], Backslash: [2, 14], Delete: [2, 15],
  End: [2, 16], PageDown: [2, 17], CapsLock: [3, 1], KeyA: [3, 2], KeyS: [3, 3],
  KeyD: [3, 4], KeyF: [3, 5], KeyG: [3, 6], KeyH: [3, 7], KeyJ: [3, 8], KeyK: [3, 9],
  KeyL: [3, 10], Semicolon: [3, 11], Quote: [3, 12], Enter: [3, 14], ShiftLeft: [4, 1],
  KeyZ: [4, 3], KeyX: [4, 4], KeyC: [4, 5], KeyV: [4, 6], KeyB: [4, 7], KeyN: [4, 8],
  KeyM: [4, 9], Comma: [4, 10], Period: [4, 11], Slash: [4, 12], ShiftRight: [4, 14],
  ArrowUp: [4, 16], ControlLeft: [5, 1], MetaLeft: [5, 2], AltLeft: [5, 3], Space: [5, 7],
  AltRight: [5, 11], FnKey: [5, 12], ContextMenu: [5, 13], ControlRight: [5, 14],
  ArrowLeft: [5, 15], ArrowDown: [5, 16], ArrowRight: [5, 17],
};

const EFFECTS = [
  ["static", "Static"], ["wave", "Wave"], ["spectrum", "Spectrum"], ["breath", "Breathing"],
  ["reactive", "Reactive"], ["fire", "Fire"], ["starlight", "Starlight"], ["ripple", "Ripple"],
  ["gradient", "Gradient"], ["marquee", "Marquee"], ["comet", "Comet"], ["off", "Off"],
];
const ANIMATED = new Set(["wave", "spectrum", "breath", "reactive", "fire", "starlight",
  "ripple", "gradient", "marquee", "comet"]);

const state = { effect: "static", color: "#ff5a1f", brightness: 80, speed: 2, direction: 1 };
let device = null;
let keys = [];
let drawing = { plate: null };

/* ------------------------------------------------------------------ reports */

function report(cls, cmd, size, args = []) {
  const r = new Uint8Array(90);
  r[0] = 0x00; r[1] = 0xff; r[5] = size; r[6] = cls; r[7] = cmd;
  args.forEach((value, index) => { r[8 + index] = value; });
  let crc = 0;
  for (let i = 2; i < 88; i++) crc ^= r[i];
  r[88] = crc;
  return r;
}

const firmwareRequest = () => report(0x00, 0x81, 0x02);
const driverMode = () => report(0x00, 0x04, 0x02, [0x03, 0x00]);
const brightness = (percent) =>
  report(0x03, 0x03, 0x03, [0x01, 0x05, Math.round((percent / 100) * 255)]);
const customEffect = () => report(0x03, 0x0a, 0x02, [0x05, 0x01]);
const offEffect = () => report(0x03, 0x0a, 0x01, [0x00]);

function frameRow(row, cells) {
  const args = [0xff, row, 0, COLS - 1];
  for (const [r, g, b] of cells) args.push(r, g, b);
  return report(0x03, 0x0b, args.length, args);
}

/* ------------------------------------------------------------------- colour */

const hexToRgb = (hex) => {
  const v = hex.replace("#", "");
  return [parseInt(v.slice(0, 2), 16) || 0, parseInt(v.slice(2, 4), 16) || 0,
    parseInt(v.slice(4, 6), 16) || 0];
};
const rgbToHex = (rgb) =>
  "#" + rgb.map((x) => Math.round(Math.max(0, Math.min(255, x))).toString(16).padStart(2, "0")).join("");
const mix = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
const mul = (c, t) => [c[0] * t, c[1] * t, c[2] * t];

function hueToRgb(hue) {
  const h = ((hue % 360) + 360) % 360 / 60;
  const sector = Math.floor(h), f = h - sector;
  switch (sector) {
    case 0: return [1, f, 0];
    case 1: return [1 - f, 1, 0];
    case 2: return [0, 1, f];
    case 3: return [0, 1 - f, 1];
    case 4: return [f, 0, 1];
    default: return [1, 0, 1 - f];
  }
}

function hash(x, y, seed) {
  let h = (x * 374761393 + y * 668265263 + seed * 1274126177) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h & 0xffff) / 65535;
}

/* ------------------------------------------------------------------ effects */

function frame(effect, time) {
  const c1 = hexToRgb(state.color);
  const speed = Math.max(0.5, state.speed);
  const out = new Array(ROWS_N * COLS);

  for (let row = 0; row < ROWS_N; row++) {
    for (let col = 0; col < COLS; col++) {
      const x = col / (COLS - 1);
      const y = row / (ROWS_N - 1);
      let color = [0, 0, 0];

      switch (effect) {
        case "static": color = c1; break;
        case "wave": {
          const head = state.direction === 2 ? 1 - (time * 0.13 * speed) % 1 : (time * 0.13 * speed) % 1;
          const d = x - head;
          color = mul(c1, 0.08 + Math.exp(-(d * d) * 14) * 0.92);
          break;
        }
        case "spectrum": {
          const rgb = hueToRgb(x * 260 + time * 26 * speed);
          color = rgb.map((v) => v * 255);
          break;
        }
        case "breath": {
          const pulse = 0.02 + 0.98 * (0.5 - 0.5 * Math.cos(time * 1.1 * speed));
          color = mul(c1, pulse);
          break;
        }
        case "reactive": {
          const bucket = Math.floor(time * 3 * speed);
          const spark = hash(col, row, bucket);
          color = mul(c1, spark > 0.9 ? 1 : 0.12);
          break;
        }
        case "fire": {
          const flicker = hash(col, row, Math.floor(time * 9 * speed));
          const roll = Math.sin(time * 1.6 * speed + row * 0.55 + col * 0.21);
          const heat = (flicker * 0.65 + (roll * 0.5 + 0.5) * 0.35) * (1 - y * 0.45) ** 2;
          color = mul([255, 90 + 120 * heat, 20 * heat], Math.min(1, heat * 1.15));
          break;
        }
        case "starlight": {
          const spark = hash(col, row, Math.floor(time * 2.2 * speed));
          color = mul(c1, spark > 0.94 ? (spark - 0.94) / 0.06 : 0.03);
          break;
        }
        case "ripple": {
          const dx = col - (COLS - 1) / 2;
          const dy = (row - (ROWS_N - 1) / 2) * 1.7;
          const wave = Math.sin(Math.hypot(dx, dy) * 0.85 - time * 3.4 * speed);
          color = mul(c1, 0.06 + Math.pow(wave * 0.5 + 0.5, 2.2) * 0.94);
          break;
        }
        case "gradient": {
          const drift = (time * 0.12 * speed) % 1;
          const position = (x + drift) % 1;
          const t = position < 0.5 ? position * 2 : (1 - position) * 2;
          color = mix(c1, [10, 132, 255], t);
          break;
        }
        case "marquee": {
          const head = ((time * 0.32 * speed) % 1.4) - 0.2;
          const intensity = Math.max(0, 1 - Math.abs(x - head) * 3.2);
          color = mix(mul(c1, 0.05), c1, intensity * intensity * (3 - 2 * intensity));
          break;
        }
        case "comet": {
          const head = ((time * 0.42 * speed) % 1.3) - 0.15;
          const behind = head - x;
          const trail = behind >= 0 ? Math.exp(-behind * 5) : 0;
          color = mix(mul(c1, 0.03), c1, Math.max(trail, Math.abs(x - head) < 0.06 ? 1 : 0));
          break;
        }
        default: color = [0, 0, 0];
      }

      const scale = state.brightness / 100;
      out[row * COLS + col] = color.map((v) => Math.max(0, Math.min(255, Math.round(v * scale))));
    }
  }
  return out;
}

/* -------------------------------------------------------------------- board */

function buildBoard() {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.getElementById("keyboard");
  const frag = document.createDocumentFragment();
  const width = BOARD_W + CASE.pad * 2;
  const height = BOARD_H + CASE.top + CASE.lip;

  const shell = document.createElementNS(ns, "rect");
  shell.setAttribute("class", "case");
  for (const [key, value] of Object.entries({ x: 0, y: 0, width, height, rx: 18 })) {
    shell.setAttribute(key, value);
  }
  frag.append(shell);

  const deck = document.createElementNS(ns, "rect");
  deck.setAttribute("class", "deck");
  for (const [key, value] of Object.entries({
    x: CASE.pad - 11, y: CASE.top - 11,
    width: BOARD_W + 22, height: BOARD_H + 22, rx: 10,
  })) deck.setAttribute(key, value);
  frag.append(deck);
  drawing.plate = deck;

  const layer = document.createElementNS(ns, "g");
  layer.setAttribute("transform", `translate(${CASE.pad} ${CASE.top})`);
  keys = [];

  const draw = (code, label, x, y, w, shift) => {
    const group = document.createElementNS(ns, "g");
    group.setAttribute("class", "key");
    group.dataset.code = code;

    const well = document.createElementNS(ns, "rect");
    well.setAttribute("class", "well");
    for (const [key, value] of Object.entries({ x, y, width: w, height: U, rx: 6 })) {
      well.setAttribute(key, value);
    }
    group.append(well);

    const cap = document.createElementNS(ns, "rect");
    cap.setAttribute("class", "cap");
    for (const [key, value] of Object.entries({ x: x + 1, y: y + 1, width: w - 2, height: U - 2, rx: 5 })) {
      cap.setAttribute(key, value);
    }
    group.append(cap);

    if (label) {
      const text = document.createElementNS(ns, "text");
      text.setAttribute("class", "legend");
      text.setAttribute("x", x + w / 2);
      text.setAttribute("y", y + (shift ? U / 2 + 7 : U / 2 + 4.8));
      text.setAttribute("text-anchor", "middle");
      text.setAttribute("font-size", shift ? 12.5 : label.length > 4 ? 10.5 : label.length > 2 ? 12 : 13.5);
      text.textContent = label;
      group.append(text);
      keys.push({ code, legend: text, cell: KEY_CELLS[code] });
    } else {
      keys.push({ code, legend: null, cell: KEY_CELLS[code] });
    }

    if (shift) {
      const secondary = document.createElementNS(ns, "text");
      secondary.setAttribute("class", "legend shift");
      secondary.setAttribute("x", x + w / 2);
      secondary.setAttribute("y", y + 12.5);
      secondary.setAttribute("text-anchor", "middle");
      secondary.textContent = shift;
      group.append(secondary);
    }

    layer.append(group);
  };

  ROWS.forEach((row, index) => {
    let cursor = 0;
    for (const item of row.keys) {
      if (item.gap) { cursor += item.gap; continue; }
      draw(item.code, item.label, cursor * STEP, index * STEP, item.w * U + (item.w - 1) * GAP, item.shift);
      cursor += item.w;
    }
  });
  for (const item of NAV) draw(item.code, item.label, item.c * STEP, item.r * STEP, U);

  frag.append(layer);
  svg.replaceChildren(frag);
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
}

function paintPreview(cells) {
  for (const entry of keys) {
    if (!entry.legend || !entry.cell) continue;
    const [row, col] = entry.cell;
    const [r, g, b] = cells[row * COLS + col];
    entry.legend.style.fill = `rgb(${r},${g},${b})`;
    entry.legend.style.opacity = String(0.35 + 0.65 * Math.max(r, g, b) / 255);
  }
}

/* --------------------------------------------------------------- transport */

async function connect() {
  if (!navigator.hid) {
    alert("This browser has no WebHID. Use Chrome, Edge or Brave.");
    return;
  }
  const devices = await navigator.hid.requestDevice({ filters: [{ vendorId: 0x1532 }] });
  for (const candidate of devices) {
    try {
      await candidate.open();
      await candidate.sendFeatureReport(0, firmwareRequest());
      const response = await candidate.receiveFeatureReport(0);
      const bytes = new Uint8Array(response.buffer ?? response);
      if (bytes[0] !== 0x02) continue;
      device = candidate;
      setStatus(true, `${candidate.productName} · fw ${bytes[8]}.${String(bytes[9]).padStart(2, "0")}`);
      await candidate.sendFeatureReport(0, driverMode());
      return;
    } catch {
      /* this interface refuses writes — try the next one */
    }
  }
  setStatus(false, "No interface accepted the report");
}

function setStatus(connected, text) {
  document.getElementById("chip-led").dataset.on = String(connected);
  document.getElementById("chip-status").textContent = text;
  document.getElementById("device-tab").textContent = connected ? "Keyboard linked" : "No keyboard linked";
}

async function send(bytes) {
  if (!device) return;
  try {
    await device.sendFeatureReport(0, bytes);
  } catch (err) {
    setStatus(false, "Write failed — reconnect");
  }
}

/* ---------------------------------------------------------------------- ui */

let lastEffect = null;
let raf = null;

async function pushFrame(time) {
  const cells = frame(state.effect, time);
  paintPreview(cells);

  if (device) {
    if (state.effect === "off") {
      await send(offEffect());
      return;
    }
    if (lastEffect !== state.effect) {
      await send(customEffect());
      lastEffect = state.effect;
    }
    for (let row = 0; row < ROWS_N; row++) {
      await send(frameRow(row, cells.slice(row * COLS, (row + 1) * COLS)));
    }
  }
}

function loop(now) {
  pushFrame(now / 1000);
  if (ANIMATED.has(state.effect)) {
    raf = requestAnimationFrame(loop);
  } else {
    raf = null;
  }
}

function restart() {
  if (raf) cancelAnimationFrame(raf);
  raf = null;
  lastEffect = null;
  if (device) send(brightness(state.brightness));
  loop(performance.now());
}

function buildUi() {
  const effects = document.getElementById("effects");
  for (const [id, name] of EFFECTS) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "effect";
    button.dataset.effect = id;
    button.setAttribute("role", "radio");
    button.setAttribute("aria-checked", String(id === state.effect));
    button.innerHTML = `<span class="effect-name">${name}</span>`;
    button.addEventListener("click", () => {
      state.effect = id;
      for (const other of effects.querySelectorAll(".effect")) {
        other.setAttribute("aria-checked", String(other.dataset.effect === id));
      }
      document.getElementById("direction-row").hidden = id !== "wave";
      document.getElementById("speed-row").hidden = !ANIMATED.has(id);
      restart();
    });
    effects.append(button);
  }

  const palette = document.getElementById("palette");
  for (const color of ["#ff3b30", "#ff9500", "#ffcc00", "#44d62c", "#00c7be",
    "#0a84ff", "#5e5ce6", "#bf5af2", "#ff2d55", "#f2f4ef"]) {
    const dot = document.createElement("button");
    dot.type = "button";
    dot.className = "dot";
    dot.style.background = color;
    dot.dataset.color = color;
    dot.setAttribute("aria-label", `Use ${color}`);
    dot.addEventListener("click", () => {
      state.color = color;
      document.getElementById("hex").value = color.slice(1).toUpperCase();
      restart();
    });
    palette.append(dot);
  }

  document.getElementById("hex").addEventListener("change", (event) => {
    const value = event.target.value.trim().replace("#", "");
    if (/^[0-9a-f]{6}$/i.test(value)) {
      state.color = `#${value.toLowerCase()}`;
      restart();
    }
  });

  document.getElementById("brightness").addEventListener("input", (event) => {
    state.brightness = Number(event.target.value);
    document.getElementById("brightness-out").value = String(state.brightness);
    restart();
  });

  document.getElementById("direction").addEventListener("click", (event) => {
    const button = event.target.closest("button");
    if (!button) return;
    state.direction = Number(button.dataset.direction);
    for (const other of event.currentTarget.querySelectorAll("button")) {
      other.setAttribute("aria-pressed", String(other === button));
    }
    restart();
  });

  document.getElementById("speed").addEventListener("click", (event) => {
    const button = event.target.closest("button");
    if (!button) return;
    state.speed = Number(button.dataset.speed);
    for (const other of event.currentTarget.querySelectorAll("button")) {
      other.setAttribute("aria-pressed", String(other === button));
    }
    restart();
  });

  document.getElementById("connect").addEventListener("click", connect);
}

/* -------------------------------------------------------------------- boot */

async function boot() {
  if (!navigator.hid) {
    setStatus(false, "WebHID unavailable — use Chrome, Edge or Brave");
    document.getElementById("alert").hidden = false;
    document.getElementById("alert").textContent =
      "This browser cannot reach USB devices. Lighting needs WebHID: Chrome, Edge or Brave on macOS.";
  }
  buildBoard();
  buildUi();
  document.getElementById("hex").value = state.color.slice(1).toUpperCase();

  const known = await navigator.hid?.getDevices?.();
  const granted = (known || []).find((entry) => entry.vendorId === 0x1532);
  if (granted) {
    try {
      await granted.open();
      await granted.sendFeatureReport(0, firmwareRequest());
      const response = await granted.receiveFeatureReport(0);
      const bytes = new Uint8Array(response.buffer ?? response);
      if (bytes[0] === 0x02) {
        device = granted;
        setStatus(true, `${granted.productName} · fw ${bytes[8]}.${String(bytes[9]).padStart(2, "0")}`);
        await granted.sendFeatureReport(0, driverMode());
      }
    } catch { /* needs a fresh pick */ }
  }

  restart();
}

boot();
