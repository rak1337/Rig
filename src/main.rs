mod devices;
mod frontmost;
mod hid;
mod input;
mod keys;
mod render;
mod report;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tiny_http::{Header, Method, Response, Server, StatusCode};

const INDEX_HTML: &str = include_str!("../web/index.html");
const APP_CSS: &str = include_str!("../web/app.css");
const APP_JS: &str = include_str!("../web/app.js");
const WEBHID_HTML: &str = include_str!("../web/webhid.html");
const FONT_ARCHIVO: &[u8] = include_bytes!("../web/fonts/Archivo.ttf");
const FONT_MONO: &[u8] = include_bytes!("../web/fonts/IBMPlexMono.ttf");
const FAVICON_SVG: &str = include_str!("../assets/logo.svg");

const DEFAULT_PORT: u16 = 7311;
const PRODUCT_ID: u16 = devices::DEFAULT_PID;

// ---------------------------------------------------------------------------
// state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct LightState {
    effect: String,
    color: String,
    color2: String,
    brightness: u8,
    speed: u8,
    direction: u8,
    breath: String,
    paint: BTreeMap<String, String>,
    driver: bool,
    game_mode: bool,
    macro_mode: bool,
    #[serde(default)]
    idle_minutes: u32,
    #[serde(default = "one")]
    gamma: f32,
    #[serde(default = "one")]
    gain_r: f32,
    #[serde(default = "one")]
    gain_g: f32,
    #[serde(default = "one")]
    gain_b: f32,
}

fn one() -> f32 {
    1.0
}

impl Default for LightState {
    fn default() -> Self {
        LightState {
            effect: "static".into(),
            color: "#ffffff".into(),
            color2: "#0a84ff".into(),
            brightness: 80,
            speed: 2,
            direction: 1,
            breath: "single".into(),
            paint: BTreeMap::new(),
            driver: true,
            game_mode: false,
            macro_mode: false,
            idle_minutes: 0,
            gamma: 1.0,
            gain_r: 1.0,
            gain_g: 1.0,
            gain_b: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Preset {
    id: u64,
    name: String,
    state: LightState,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct KeysStore {
    #[serde(default)]
    remaps: Vec<(u16, u16)>,
    #[serde(default)]
    remaps2: Vec<(u16, u16)>,
    #[serde(default)]
    hypershift: Option<u16>,
    #[serde(default)]
    macros: Vec<keys::Macro>,
    #[serde(default)]
    next_macro: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Profile {
    id: u64,
    name: String,
    #[serde(default)]
    bundle: Option<String>,
    state: LightState,
    #[serde(default)]
    remaps: Vec<(u16, u16)>,
}

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    state: LightState,
    presets: Vec<Preset>,
    next_id: u64,
    #[serde(default)]
    keys: KeysStore,
    #[serde(default)]
    profiles: Vec<Profile>,
    #[serde(default)]
    next_profile: u64,
    #[serde(default)]
    active_profile: Option<u64>,
    #[serde(default)]
    revision: u64,
}

struct App {
    stored: Mutex<Stored>,
    keyboard: Mutex<Option<hid::Keyboard>>,
    firmware: Mutex<Option<String>>,
    renderer: Mutex<render::Settings>,
    frame: Mutex<Vec<(u8, u8, u8)>>,
    engine: keys::Engine,
    pid: u16,
    /// Diagnostics posted by the browser, so a rendering failure on the client
    /// is not invisible from the terminal.
    logs: Mutex<Vec<String>>,
}

impl App {
    fn load(pid: u16) -> App {
        let stored = std::fs::read_to_string(store_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<Stored>(&raw).ok())
            .unwrap_or_else(|| Stored {
                state: LightState::default(),
                presets: Vec::new(),
                next_id: 1,
                keys: KeysStore::default(),
                profiles: Vec::new(),
                next_profile: 1,
                active_profile: None,
                revision: 0,
            });
        App {
            stored: Mutex::new(stored),
            keyboard: Mutex::new(None),
            firmware: Mutex::new(None),
            renderer: Mutex::new(render::Settings::default()),
            frame: Mutex::new(Vec::new()),
            engine: keys::Engine::new(),
            pid,
            logs: Mutex::new(Vec::new()),
        }
    }

    fn save(&self) {
        let stored = self.stored.lock().unwrap();
        let Ok(json) = serde_json::to_string_pretty(&*stored) else {
            return;
        };
        let path = store_path();
        if let Some(dir) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(dir) {
                eprintln!(
                    "rig: could not create {}: {err}",
                    dir.display()
                );
                return;
            }
        }
        // A read-only home is worth a word rather than a silent loss of presets.
        if let Err(err) = std::fs::write(&path, json) {
            eprintln!(
                "rig: could not save state to {}: {err}\n\
                 presets will not persist until that path is writable",
                path.display()
            );
        }
    }

    /// Push the state to the keyboard. Returns the firmware string on success.
    fn apply(&self, state: &LightState) -> Result<String, String> {
        let mut guard = self.keyboard.lock().unwrap();
        if guard.is_none() {
            *guard = Some(hid::Keyboard::open(self.pid)?);
        }
        {
            let mut renderer = self.renderer.lock().unwrap();
            if render::is_software(&state.effect) {
                renderer.active = true;
                renderer.effect = state.effect.clone();
                renderer.c1 = parse_color(&state.color).unwrap_or((255, 255, 255));
                renderer.c2 = parse_color(&state.color2).unwrap_or((10, 132, 255));
                renderer.speed = state.speed.max(1) as f32;
                renderer.brightness = 1.0;
                renderer.direction = state.direction;
                renderer.breath = state.breath.clone();
                renderer.idle_seconds = state.idle_minutes.saturating_mul(60);
                renderer.gamma = state.gamma;
                renderer.gain = (state.gain_r, state.gain_g, state.gain_b);
            } else {
                renderer.active = false;
            }
        }

        let outcome = push_state(guard.as_ref().expect("just opened"), state);
        if outcome.is_err() {
            *guard = None; // force a reopen on the next attempt
            outcome?;
        }

        let mut firmware_guard = self.firmware.lock().unwrap();
        if firmware_guard.is_none() {
            let response = guard
                .as_ref()
                .expect("open")
                .exchange(&report::firmware_version())?;
            *firmware_guard = Some(format!("{}.{:02}", response[8], response[9]));
        }
        Ok(firmware_guard.clone().unwrap_or_default())
    }

    fn activate_profile(&self, id: u64) {
        let profile = {
            let stored = self.stored.lock().unwrap();
            stored.profiles.iter().find(|item| item.id == id).cloned()
        };
        let Some(profile) = profile else { return };
        let state = profile.state.clone();
        {
            let mut stored = self.stored.lock().unwrap();
            stored.state = state.clone();
            stored.keys.remaps = profile.remaps.clone();
            stored.active_profile = Some(id);
            stored.revision += 1;
        }
        self.sync_remaps();
        if let Err(err) = self.apply(&state) {
            eprintln!("rig: profile {} could not be applied: {err}", profile.name);
        }
        self.save();
    }

    fn profile_for_bundle(&self, bundle: &str) -> Option<u64> {
        let stored = self.stored.lock().unwrap();
        stored
            .profiles
            .iter()
            .find(|item| item.bundle.as_deref() == Some(bundle))
            .map(|item| item.id)
    }

    fn active_profile(&self) -> Option<u64> {
        self.stored.lock().unwrap().active_profile
    }

    fn profiles_payload(&self) -> ProfilesPayload {
        let stored = self.stored.lock().unwrap();
        ProfilesPayload {
            active: stored.active_profile,
            frontmost: frontmost::frontmost_bundle(),
            profiles: stored.profiles.clone(),
        }
    }

    fn keys_payload(&self) -> KeysPayload {
        let stored = self.stored.lock().unwrap();
        KeysPayload {
            trusted: self.engine.trusted(),
            tap: self.engine.tap_active(),
            recording: self.engine.is_recording(),
            steps: self.engine.recorded_len(),
            remaps: stored.keys.remaps.clone(),
            remaps2: stored.keys.remaps2.clone(),
            hypershift: stored.keys.hypershift,
            macros: stored.keys.macros.clone(),
        }
    }

    fn sync_triggers(&self) {
        let triggers: Vec<(u16, Vec<keys::Step>)> = {
            let stored = self.stored.lock().unwrap();
            stored
                .keys
                .macros
                .iter()
                .filter_map(|item| item.trigger.map(|key| (key, item.steps.clone())))
                .collect()
        };
        self.engine.set_triggers(&triggers);
    }

    fn sync_remaps(&self) {
        let (remaps, layer2, hyper) = {
            let stored = self.stored.lock().unwrap();
            (
                stored.keys.remaps.clone(),
                stored.keys.remaps2.clone(),
                stored.keys.hypershift,
            )
        };
        self.engine.set_remaps(&remaps);
        self.engine.set_hypershift(hyper, &layer2);
    }

    /// Try to hold a device handle, returning why it failed (if it did).
    fn ensure_device(&self) -> Option<String> {
        let mut guard = self.keyboard.lock().unwrap();
        if guard.is_some() {
            return None;
        }
        match hid::Keyboard::open(self.pid) {
            Ok(keyboard) => {
                *guard = Some(keyboard);
                None
            }
            Err(err) => Some(err),
        }
    }

    fn payload(&self, error: Option<String>) -> Payload {
        let stored = self.stored.lock().unwrap();
        Payload {
            revision: stored.revision,
            device: devices::info(self.pid),
            connected: self.keyboard.lock().unwrap().is_some(),
            firmware: self.firmware.lock().unwrap().clone(),
            error,
            state: stored.state.clone(),
            presets: stored.presets.clone(),
        }
    }
}

fn push_state(keyboard: &hid::Keyboard, state: &LightState) -> Result<String, String> {
    if state.driver {
        check(keyboard.exchange(&report::device_mode(0x03))?, "driver mode")?;
    }
    check(keyboard.exchange(&report::brightness(state.brightness))?, "brightness")?;
    check(
        keyboard.exchange(&report::indicator(state.game_mode, true))?,
        "game mode",
    )?;
    check(
        keyboard.exchange(&report::indicator(state.macro_mode, false))?,
        "macro led",
    )?;

    match state.effect.as_str() {
        // The two effects the board itself still owns.
        "off" => check(keyboard.exchange(&report::off())?, "off")?,
        "paint" => push_paint(keyboard, state)?,
        // Everything else is drawn by the render thread; sending a firmware
        // effect here would fight it.
        effect if render::is_software(effect) => {}
        other => return Err(format!("unknown effect {other:?}")),
    }
    Ok(String::new())
}

fn push_paint(keyboard: &hid::Keyboard, state: &LightState) -> Result<(), String> {
    for row in 0..report::MATRIX_ROWS {
        let mut colors = vec![(0u8, 0u8, 0u8); report::MATRIX_COLS];
        for col in 0..report::MATRIX_COLS {
            let key = format!("{row},{col}");
            if let Some(hex) = state.paint.get(&key) {
                if let Some(color) = parse_color(hex) {
                    colors[col] = color;
                }
            }
        }
        check(keyboard.exchange(&report::custom_frame_row(row, &colors))?, "frame")?;
    }
    check(keyboard.exchange(&report::custom_effect())?, "custom effect")?;
    Ok(())
}

fn check(response: [u8; 90], what: &str) -> Result<(), String> {
    match response[0] {
        report::STATUS_OK => Ok(()),
        report::STATUS_BUSY => Err(format!("{what}: device busy")),
        status => Err(format!("{what}: device rejected the command (0x{status:02x})")),
    }
}

fn parse_color(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn store_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join("Library/Application Support/Rig")
        .join("state.json")
}

// ---------------------------------------------------------------------------
// payloads
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Payload {
    revision: u64,
    device: devices::DeviceInfo,
    connected: bool,
    firmware: Option<String>,
    error: Option<String>,
    state: LightState,
    presets: Vec<Preset>,
}

#[derive(Deserialize)]
struct PresetRequest {
    name: String,
    state: LightState,
}

#[derive(Serialize, Deserialize)]
struct Backup {
    state: LightState,
    #[serde(default)]
    presets: Vec<Preset>,
    #[serde(default)]
    profiles: Vec<Profile>,
    #[serde(default)]
    keys: KeysStore,
}

#[derive(Serialize)]
struct ProfilesPayload {
    active: Option<u64>,
    frontmost: Option<String>,
    profiles: Vec<Profile>,
}

#[derive(Serialize)]
struct KeysPayload {
    trusted: bool,
    tap: bool,
    recording: bool,
    steps: usize,
    remaps: Vec<(u16, u16)>,
    remaps2: Vec<(u16, u16)>,
    hypershift: Option<u16>,
    macros: Vec<keys::Macro>,
}

#[derive(Deserialize)]
struct RemapRequest {
    from: u16,
    to: u16,
    #[serde(default)]
    layer: u8,
}

#[derive(Deserialize)]
struct MacroName {
    name: String,
}

#[derive(Deserialize)]
struct IdRequest {
    id: u64,
}

// ---------------------------------------------------------------------------
// http
// ---------------------------------------------------------------------------

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap()
}

fn respond_json(request: tiny_http::Request, body: &impl Serialize, status: u16) {
    let text = serde_json::to_string(body).unwrap_or_else(|_| "{}".into());
    let response = Response::from_string(text)
        .with_status_code(StatusCode(status))
        .with_header(json_header())
        .with_header(Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap());
    let _ = request.respond(response);
}

fn respond_bytes(request: tiny_http::Request, bytes: Vec<u8>, content_type: &'static str) {
    let response = Response::from_data(bytes)
        .with_header(Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap())
        .with_header(Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap());
    let _ = request.respond(response);
}

fn read_json<T: for<'de> Deserialize<'de>>(request: &mut tiny_http::Request) -> Option<T> {
    let mut body = String::new();
    request.as_reader().take(1 << 20).read_to_string(&mut body).ok()?;
    serde_json::from_str(&body).ok()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--test") {
        std::process::exit(match smoke_test() {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("error: {err}");
                1
            }
        });
    }
    if args.iter().any(|a| a == "--probe") {
        std::process::exit(match probe() {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("error: {err}");
                1
            }
        });
    }
    if args.iter().any(|a| a == "--install-login") {
        std::process::exit(match install_login() {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("error: {err}");
                1
            }
        });
    }
    if args.iter().any(|a| a == "--uninstall-login") {
        std::process::exit(match uninstall_login() {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("error: {err}");
                1
            }
        });
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "rig {} — a control console for Razer keyboards on macOS\n\n\
             usage: rig [--port 7311] [--no-open] [--no-restore] [--pid 0209]\n\
             \x20          [--host 127.0.0.1] [--token <t>]\n\
             \x20      rig --probe | --test\n\
             \x20      rig --install-login | --uninstall-login\n\n\
             --port <n>         serve the console on another port (default 7311)\n\
             --no-open          do not open the browser on launch\n\
             --no-restore       do not re-apply the saved state on launch\n\
             --pid <hex>        target another Razer product id (default 0209)\n\
             --host <addr>      listen beyond loopback; requires a token\n\
             --token <t>        token for --host (or RIG_TOKEN; random if unset)\n\
             --probe            read firmware, device mode, brightness and the\n\
             \x20                  indicator LEDs straight from the board\n\
             --test             set static white and read the firmware version\n\
             --install-login    start at login (needed only for animated effects\n\
             \x20                  and key features; still looks stay on the board)\n\
             --uninstall-login  remove the login item\n\n\
             Serves the console at http://127.0.0.1:7311. State and presets live in\n\
             ~/Library/Application Support/Rig/state.json",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }

    let port = flag_value(&args, "--port")
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let open_browser = !args.iter().any(|a| a == "--no-open");
    let restore = !args.iter().any(|a| a == "--no-restore");

    let pid = flag_value(&args, "--pid")
        .and_then(|value| u16::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        .unwrap_or(PRODUCT_ID);
    let app = Arc::new(App::load(pid));
    app.sync_remaps();
    app.sync_triggers();
    app.engine.start();
    spawn_renderer(Arc::clone(&app));
    spawn_watcher(Arc::clone(&app));
    spawn_hotkeys(Arc::clone(&app));

    let host = flag_value(&args, "--host").unwrap_or_else(|| "127.0.0.1".into());
    let loopback = host == "127.0.0.1" || host == "localhost" || host == "::1";
    let token = match flag_value(&args, "--token").or_else(|| std::env::var("RIG_TOKEN").ok()) {
        Some(value) => Some(value),
        // Anything that is not loopback drives real hardware, so it needs a key.
        None if !loopback => Some(random_token()),
        None => None,
    };

    let store = match Server::http((host.as_str(), port)) {
        Ok(server) => server,
        Err(err) => {
            eprintln!("rig: cannot listen on {host}:{port}: {err}");
            std::process::exit(1);
        }
    };

    let url = format!("http://{host}:{port}");
    match &token {
        Some(token) => {
            println!("rig listening on {url}");
            println!("open this to sign in: {url}/?t={token}");
        }
        None => println!("rig listening on {url}"),
    }
    if open_browser {
        let target = match &token {
            Some(token) => format!("{url}/?t={token}"),
            None => url.clone(),
        };
        let _ = std::process::Command::new("open").arg(target).spawn();
    }

    if restore {
        let state = app.stored.lock().unwrap().state.clone();
        match app.apply(&state) {
            Ok(firmware) => println!("restored {} lighting (firmware {firmware})", state.effect),
            Err(err) => println!("note: {err}"),
        }
    }

    for mut request in store.incoming_requests() {
        if let Some(expected) = token.as_deref() {
            let url_now = request.url().to_string();
            let supplied = query_param(&url_now, "t")
                .or_else(|| cookie_value(&request, "rig_token"));

            if supplied.as_deref() != Some(expected) {
                let body = "<!doctype html><meta charset=utf-8><title>Rig</title>\
                    <body style=\"margin:0;height:100vh;display:grid;place-items:center;\
                    background:#141414;color:#ddd;font:14px system-ui\">\
                    <div style=\"text-align:center\"><h1 style=\"font-size:15px;\
                    letter-spacing:.14em;text-transform:uppercase\">Rig</h1>\
                    <p>Open the link printed in the terminal — it carries the access token.</p></div>";
                let _ = request.respond(
                    Response::from_string(body)
                        .with_status_code(StatusCode(401))
                        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()),
                );
                continue;
            }

            if query_param(&url_now, "t").is_some() {
                // Trade the token in the URL for a cookie and get it out of the bar.
                let path = url_now.split('?').next().unwrap_or("/").to_string();
                let cookie = format!("rig_token={expected}; Path=/; HttpOnly; SameSite=Lax");
                let _ = request.respond(
                    Response::empty(StatusCode(302))
                        .with_header(Header::from_bytes(&b"Location"[..], path.as_bytes()).unwrap())
                        .with_header(Header::from_bytes(&b"Set-Cookie"[..], cookie.as_bytes()).unwrap()),
                );
                continue;
            }
        }

        let path = request.url().split('?').next().unwrap_or("/").to_string();
        let method = request.method().clone();

        match (&method, path.as_str()) {
            (Method::Get, "/") | (Method::Get, "/index.html") => {
                respond_bytes(request, INDEX_HTML.as_bytes().to_vec(), "text/html; charset=utf-8")
            }
            (Method::Get, "/app.css") => {
                respond_bytes(request, APP_CSS.as_bytes().to_vec(), "text/css; charset=utf-8")
            }
            (Method::Get, "/app.js") => respond_bytes(
                request,
                APP_JS.as_bytes().to_vec(),
                "text/javascript; charset=utf-8",
            ),
            (Method::Get, "/fonts/archivo.ttf") => {
                respond_bytes(request, FONT_ARCHIVO.to_vec(), "font/ttf")
            }
            (Method::Get, "/favicon.svg") => {
                respond_bytes(request, FAVICON_SVG.as_bytes().to_vec(), "image/svg+xml")
            }
            (Method::Get, "/fonts/plex-mono.ttf") => {
                respond_bytes(request, FONT_MONO.to_vec(), "font/ttf")
            }
            (Method::Get, "/api/frame") => {
                let frame = app.frame.lock().unwrap().clone();
                let hexes: Vec<String> = frame
                    .iter()
                    .map(|(r, g, b)| format!("#{r:02x}{g:02x}{b:02x}"))
                    .collect();
                respond_json(request, &hexes, 200);
            }
            (Method::Get, "/api/state") => {
                let error = app.ensure_device();
                let payload = app.payload(error);
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/apply") => {
                let Some(state) = read_json::<LightState>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                let outcome = app.apply(&state);
                {
                    let mut stored = app.stored.lock().unwrap();
                    stored.state = state.clone();
                    stored.revision += 1;
                }
                app.save();
                match outcome {
                    Ok(_) => {
                        let payload = app.payload(None);
                        respond_json(request, &payload, 200);
                    }
                    Err(err) => {
                        *app.keyboard.lock().unwrap() = None;
                        let payload = app.payload(Some(err));
                        respond_json(request, &payload, 200);
                    }
                }
            }
            (Method::Post, "/api/presets") => {
                let Some(req) = read_json::<PresetRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    let id = stored.next_id;
                    stored.next_id += 1;
                    stored.presets.push(Preset {
                        id,
                        name: req.name.trim().to_string(),
                        state: req.state,
                    });
                    if stored.presets.len() > 24 {
                        stored.presets.remove(0);
                    }
                }
                app.save();
                let payload = app.payload(None);
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/presets/delete") => {
                let Some(req) = read_json::<IdRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    stored.presets.retain(|preset| preset.id != req.id);
                }
                app.save();
                let payload = app.payload(None);
                respond_json(request, &payload, 200);
            }
            (Method::Get, "/api/profiles") => {
                let payload = app.profiles_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/profiles") => {
                #[derive(Deserialize)]
                struct NewProfile {
                    name: String,
                    #[serde(default)]
                    bundle: Option<String>,
                }
                let Some(req) = read_json::<NewProfile>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    let id = if stored.next_profile == 0 { 1 } else { stored.next_profile };
                    stored.next_profile = id + 1;
                    let state = stored.state.clone();
                    let remaps = stored.keys.remaps.clone();
                    stored.profiles.push(Profile {
                        id,
                        name: req.name.trim().to_string(),
                        bundle: req.bundle,
                        state,
                        remaps,
                    });
                }
                app.save();
                let payload = app.profiles_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/profiles/activate") => {
                let Some(req) = read_json::<IdRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                app.activate_profile(req.id);
                let payload = app.profiles_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/profiles/delete") => {
                let Some(req) = read_json::<IdRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    stored.profiles.retain(|item| item.id != req.id);
                    if stored.active_profile == Some(req.id) {
                        stored.active_profile = None;
                    }
                }
                app.save();
                let payload = app.profiles_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/profiles/bind") => {
                let Some(req) = read_json::<IdRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                let bundle = frontmost::frontmost_bundle();
                {
                    let mut stored = app.stored.lock().unwrap();
                    if let Some(item) = stored.profiles.iter_mut().find(|item| item.id == req.id) {
                        item.bundle = bundle;
                    }
                }
                app.save();
                let payload = app.profiles_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/log") => {
                #[derive(Deserialize)]
                struct Entry {
                    message: String,
                }
                if let Some(entry) = read_json::<Entry>(&mut request) {
                    let line = format!("client: {}", entry.message);
                    eprintln!("{line}");
                    let mut logs = app.logs.lock().unwrap();
                    logs.push(line);
                    if logs.len() > 200 {
                        logs.remove(0);
                    }
                }
                respond_json(request, &serde_json::json!({"ok": true}), 200);
            }
            (Method::Get, "/webhid") => {
                respond_bytes(request, WEBHID_HTML.as_bytes().to_vec(), "text/html; charset=utf-8")
            }
            (Method::Get, "/api/log") => {
                let logs = app.logs.lock().unwrap().clone();
                respond_json(request, &logs, 200);
            }
            (Method::Get, "/api/export") => {
                let backup = {
                    let stored = app.stored.lock().unwrap();
                    Backup {
                        state: stored.state.clone(),
                        presets: stored.presets.clone(),
                        profiles: stored.profiles.clone(),
                        keys: stored.keys.clone(),
                    }
                };
                respond_json(request, &backup, 200);
            }
            (Method::Post, "/api/import") => {
                let Some(backup) = read_json::<Backup>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    stored.state = backup.state.clone();
                    stored.presets = backup.presets;
                    stored.profiles = backup.profiles;
                    stored.keys = backup.keys;
                    stored.revision += 1;
                }
                app.sync_remaps();
                app.sync_triggers();
                app.save();
                let _ = app.apply(&backup.state);
                let payload = app.payload(None);
                respond_json(request, &payload, 200);
            }
            (Method::Get, "/api/keys") => {
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/remap") => {
                let Some(req) = read_json::<RemapRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    let table = if req.layer == 2 {
                        &mut stored.keys.remaps2
                    } else {
                        &mut stored.keys.remaps
                    };
                    table.retain(|(from, _)| *from != req.from);
                    table.push((req.from, req.to));
                }
                app.sync_remaps();
                app.save();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/remap/delete") => {
                let Some(req) = read_json::<RemapRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    let table = if req.layer == 2 {
                        &mut stored.keys.remaps2
                    } else {
                        &mut stored.keys.remaps
                    };
                    table.retain(|(from, _)| *from != req.from);
                }
                app.sync_remaps();
                app.save();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/record/start") => {
                app.engine.start_recording();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/record/stop") => {
                let Some(req) = read_json::<MacroName>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                let steps = app.engine.stop_recording();
                let dropped = steps.len();
                {
                    let mut stored = app.stored.lock().unwrap();
                    if !steps.is_empty() {
                        let id = if stored.keys.next_macro == 0 { 1 } else { stored.keys.next_macro };
                        stored.keys.next_macro = id + 1;
                        stored.keys.macros.push(keys::Macro {
                            id,
                            trigger: None,
                            name: if req.name.trim().is_empty() {
                                format!("Macro {id}")
                            } else {
                                req.name.trim().to_string()
                            },
                            steps,
                        });
                    }
                }
                if dropped == 0 {
                    respond_json(request, &serde_json::json!({"error": "nothing was recorded"}), 200);
                    continue;
                }
                app.sync_triggers();
                app.save();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/hypershift") => {
                #[derive(Deserialize)]
                struct HyperRequest {
                    key: Option<u16>,
                }
                let Some(req) = read_json::<HyperRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    stored.keys.hypershift = req.key;
                }
                app.sync_remaps();
                app.save();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/macro/bind") => {
                #[derive(Deserialize)]
                struct BindRequest {
                    id: u64,
                    key: u16,
                }
                let Some(req) = read_json::<BindRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    // one key, one macro
                    for item in stored.keys.macros.iter_mut() {
                        if item.trigger == Some(req.key) {
                            item.trigger = None;
                        }
                    }
                    if let Some(item) = stored.keys.macros.iter_mut().find(|item| item.id == req.id) {
                        item.trigger = Some(req.key);
                    }
                }
                app.sync_triggers();
                app.save();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            (Method::Post, "/api/keys/macro/play") => {
                let Some(req) = read_json::<IdRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                let steps = {
                    let stored = app.stored.lock().unwrap();
                    stored
                        .keys
                        .macros
                        .iter()
                        .find(|item| item.id == req.id)
                        .map(|item| item.steps.clone())
                };
                match steps {
                    Some(steps) => {
                        app.engine.play(steps);
                        let payload = app.keys_payload();
                        respond_json(request, &payload, 200);
                    }
                    None => respond_json(request, &serde_json::json!({"error": "no such macro"}), 404),
                }
            }
            (Method::Post, "/api/keys/macro/delete") => {
                let Some(req) = read_json::<IdRequest>(&mut request) else {
                    respond_json(request, &serde_json::json!({"error": "bad request"}), 400);
                    continue;
                };
                {
                    let mut stored = app.stored.lock().unwrap();
                    stored.keys.macros.retain(|item| item.id != req.id);
                }
                app.sync_triggers();
                app.save();
                let payload = app.keys_payload();
                respond_json(request, &payload, 200);
            }
            _ => {
                let _ = request.respond(Response::empty(StatusCode(404)));
            }
        }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn smoke_test() -> Result<(), String> {
    let keyboard = hid::Keyboard::open(PRODUCT_ID)?;
    let mut state = LightState::default();
    state.effect = "static".into();
    state.color = "#ffffff".into();
    push_state(&keyboard, &state)?;
    let response = keyboard.exchange(&report::firmware_version())?;
    println!("firmware {}.{:02}", response[8], response[9]);
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

// ---------------------------------------------------------------------------
// diagnostics and login item
// ---------------------------------------------------------------------------

fn probe() -> Result<(), String> {
    let keyboard = hid::Keyboard::open(PRODUCT_ID)?;

    let firmware = keyboard.exchange(&report::firmware_version())?;
    println!("firmware        {}.{:02}", firmware[8], firmware[9]);

    let mode = keyboard.exchange(&report::get_device_mode())?;
    let owner = match mode[8] {
        0x03 => "software (an app owns the LEDs)",
        0x00 => "firmware (the board runs its own effect)",
        other => return Err(format!("unexpected device mode 0x{other:02x}")),
    };
    println!("device mode     0x{:02x} — {owner}", mode[8]);

    let brightness = keyboard.exchange(&report::get_brightness())?;
    println!(
        "brightness      {}% (raw {})",
        (brightness[10] as u32 * 100 + 127) / 255,
        brightness[10]
    );

    let game = keyboard.exchange(&report::get_led_state(report::LED_GAME))?;
    println!("gaming mode     {}", on_off(game[10]));

    let macro_led = keyboard.exchange(&report::get_led_state(report::LED_MACRO))?;
    println!("macro led       {}", on_off(macro_led[10]));

    println!(
        "\nThis board has no readable profile table: Synapse profiles are a Windows\n\
         software concept, and the TE Chroma keeps no named on-board profiles.\n\
         What it can hold is the single effect currently applied, and only while\n\
         it has power."
    );
    Ok(())
}

fn on_off(value: u8) -> &'static str {
    if value == 0 { "off" } else { "on" }
}

fn launch_agent_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join("dev.rakshak.rig.plist")
}

fn install_login() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| format!("cannot find this binary: {err}"))?;
    let path = launch_agent_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    }
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>dev.rakshak.rig</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>--no-open</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>StandardOutPath</key><string>/tmp/rig.log</string>
  <key>StandardErrorPath</key><string>/tmp/rig.log</string>
</dict>
</plist>
"#,
        exe.display()
    );
    std::fs::write(&path, plist)
        .map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    println!("installed {}", path.display());
    println!("the console will start at login and re-apply the saved look");
    println!("first run needs Input Monitoring for {}", exe.display());
    Ok(())
}

fn uninstall_login() -> Result<(), String> {
    let path = launch_agent_path();
    match std::fs::remove_file(&path) {
        Ok(()) => {
            println!("removed {}", path.display());
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("no login item installed");
            Ok(())
        }
        Err(err) => Err(format!("cannot remove {}: {err}", path.display())),
    }
}

// ---------------------------------------------------------------------------
// animation thread
// ---------------------------------------------------------------------------

fn spawn_renderer(app: Arc<App>) {
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut pushed_effect = String::new();
        let mut next_open = std::time::Instant::now();

        loop {
            let settings = app.renderer.lock().unwrap().clone();
            if !settings.active {
                pushed_effect.clear();
                // Offscreen: report black rather than the last frame drawn.
                let mut stored = app.frame.lock().unwrap();
                if stored.iter().any(|(r, g, b)| *r + *g + *b > 0) {
                    *stored = vec![(0, 0, 0); render::CELLS];
                }
                drop(stored);
                std::thread::sleep(Duration::from_millis(80));
                continue;
            }

            let presses: Vec<(usize, f32)> = app
                .engine
                .drain_presses()
                .into_iter()
                .filter_map(|(key, age)| keys::cell_for_keycode(key).map(|cell| (cell, age)))
                .collect();
            let mut frame =
                render::frame(&settings, started.elapsed().as_secs_f32(), &presses);
            if settings.idle_seconds > 0
                && input::seconds_since_input() > settings.idle_seconds as f64
            {
                frame = vec![(0, 0, 0); render::CELLS];
            }
            {
                let mut stored = app.frame.lock().unwrap();
                stored.clear();
                stored.extend_from_slice(&frame);
            }

            let mut guard = app.keyboard.lock().unwrap();

            // Re-check now that we own the device: if a hardware effect was
            // selected while this frame was being computed, pushing it would
            // drag the board back into custom-frame mode and kill that effect.
            if !app.renderer.lock().unwrap().active {
                pushed_effect.clear();
                drop(guard);
                continue;
            }

            if guard.is_none() && std::time::Instant::now() >= next_open {
                *guard = hid::Keyboard::open(app.pid).ok();
                if guard.is_none() {
                    // Do not hammer a refused permission; keep previewing.
                    next_open = std::time::Instant::now() + Duration::from_secs(5);
                }
            }
            if let Some(keyboard) = guard.as_ref() {
                // Re-assert the custom mode every frame: the board drops out of it
                // when any other effect command lands, and a dropped frame would
                // otherwise leave the keyboard frozen on the last one.
                let _ = keyboard.exchange(&report::custom_effect());
                pushed_effect = settings.effect.clone();

                for row in 0..render::ROWS {
                    let cells = &frame[row * render::COLS..(row + 1) * render::COLS];
                    // Read back the status, like the paint path does; write-only
                    // frames were being dropped.
                    let _ = keyboard.exchange(&report::custom_frame_row(row, cells));
                }
            }
            drop(guard);

            std::thread::sleep(Duration::from_millis(12));
        }
    });
}

// ---------------------------------------------------------------------------
// frontmost-app watcher
// ---------------------------------------------------------------------------

fn spawn_watcher(app: Arc<App>) {
    std::thread::spawn(move || {
        let mut last = String::new();
        loop {
            if let Some(bundle) = frontmost::frontmost_bundle() {
                if bundle != last {
                    last = bundle.clone();
                    if let Some(id) = app.profile_for_bundle(&bundle) {
                        if app.active_profile() != Some(id) {
                            app.activate_profile(id);
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(1200));
        }
    });
}

// ---------------------------------------------------------------------------
// request helpers
// ---------------------------------------------------------------------------

fn query_param(url: &str, name: &str) -> Option<String> {
    let (_, query) = url.split_once('?')?;
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if key == name {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn cookie_value(request: &tiny_http::Request, name: &str) -> Option<String> {
    for header in request.headers() {
        if header.field.equiv("Cookie") {
            for part in header.value.as_str().split(';') {
                let part = part.trim();
                if let Some(value) = part.strip_prefix(&format!("{name}=")) {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn random_token() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        if file.read_exact(&mut bytes).is_ok() {
            return bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        }
    }
    format!(
        "{:x}{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

// ---------------------------------------------------------------------------
// global hotkeys
// ---------------------------------------------------------------------------

fn spawn_hotkeys(app: Arc<App>) {
    std::thread::spawn(move || loop {
        for command in app.engine.drain_commands() {
            let mut state = app.stored.lock().unwrap().state.clone();
            match command.as_str() {
                "toggle_off" => {
                    state.effect = if state.effect == "off" {
                        "static".to_string()
                    } else {
                        "off".to_string()
                    };
                }
                "next_effect" => {
                    const ORDER: [&str; 6] = ["static", "wave", "spectrum", "breath", "reactive", "fire"];
                    let index = ORDER.iter().position(|item| *item == state.effect).unwrap_or(0);
                    state.effect = ORDER[(index + 1) % ORDER.len()].to_string();
                }
                "brightness_up" => state.brightness = (state.brightness + 10).min(100),
                "brightness_down" => state.brightness = state.brightness.saturating_sub(10),
                _ => continue,
            }
            {
                let mut stored = app.stored.lock().unwrap();
                stored.state = state.clone();
                stored.revision += 1;
            }
            if let Err(err) = app.apply(&state) {
                eprintln!("rig: hotkey could not apply: {err}");
            }
            app.save();
        }
        std::thread::sleep(Duration::from_millis(80));
    });
}
