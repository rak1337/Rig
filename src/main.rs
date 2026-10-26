mod hid;
mod report;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tiny_http::{Header, Method, Response, Server, StatusCode};

const INDEX_HTML: &str = include_str!("../web/index.html");
const APP_CSS: &str = include_str!("../web/app.css");
const APP_JS: &str = include_str!("../web/app.js");
const FONT_ARCHIVO: &[u8] = include_bytes!("../web/fonts/Archivo.ttf");
const FONT_MONO: &[u8] = include_bytes!("../web/fonts/IBMPlexMono.ttf");

const DEFAULT_PORT: u16 = 7311;
const PRODUCT_ID: u16 = 0x0209;

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
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Preset {
    id: u64,
    name: String,
    state: LightState,
}

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    state: LightState,
    presets: Vec<Preset>,
    next_id: u64,
}

struct App {
    stored: Mutex<Stored>,
    keyboard: Mutex<Option<hid::Keyboard>>,
    firmware: Mutex<Option<String>>,
}

impl App {
    fn load() -> App {
        let stored = std::fs::read_to_string(store_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<Stored>(&raw).ok())
            .unwrap_or_else(|| Stored {
                state: LightState::default(),
                presets: Vec::new(),
                next_id: 1,
            });
        App {
            stored: Mutex::new(stored),
            keyboard: Mutex::new(None),
            firmware: Mutex::new(None),
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
                    "open-razor: could not create {}: {err}",
                    dir.display()
                );
                return;
            }
        }
        // A read-only home is worth a word rather than a silent loss of presets.
        if let Err(err) = std::fs::write(&path, json) {
            eprintln!(
                "open-razor: could not save state to {}: {err}\n\
                 presets will not persist until that path is writable",
                path.display()
            );
        }
    }

    /// Push the state to the keyboard. Returns the firmware string on success.
    fn apply(&self, state: &LightState) -> Result<String, String> {
        let mut guard = self.keyboard.lock().unwrap();
        if guard.is_none() {
            *guard = Some(hid::Keyboard::open(PRODUCT_ID)?);
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

    /// Try to hold a device handle, returning why it failed (if it did).
    fn ensure_device(&self) -> Option<String> {
        let mut guard = self.keyboard.lock().unwrap();
        if guard.is_some() {
            return None;
        }
        match hid::Keyboard::open(PRODUCT_ID) {
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

    let (r, g, b) = parse_color(&state.color).unwrap_or((255, 255, 255));
    match state.effect.as_str() {
        "static" => check(keyboard.exchange(&report::static_effect(r, g, b))?, "static")?,
        "wave" => check(
            keyboard.exchange(&report::wave(state.direction))?,
            "wave",
        )?,
        "spectrum" => check(keyboard.exchange(&report::spectrum())?, "spectrum")?,
        "breath" => {
            let packet = match state.breath.as_str() {
                "dual" => {
                    let (r2, g2, b2) = parse_color(&state.color2).unwrap_or((10, 132, 255));
                    report::breath_dual((r, g, b), (r2, g2, b2))
                }
                "random" => report::breath_random(),
                _ => report::breath_single(r, g, b),
            };
            check(keyboard.exchange(&packet)?, "breathing")?;
        }
        "reactive" => check(
            keyboard.exchange(&report::reactive(state.speed, r, g, b))?,
            "reactive",
        )?,
        "off" => check(keyboard.exchange(&report::off())?, "off")?,
        "paint" => push_paint(keyboard, state)?,
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
        .join("Library/Application Support/OpenRazor")
        .join("state.json")
}

// ---------------------------------------------------------------------------
// payloads
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Payload {
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
        .with_header(Header::from_bytes(&b"Cache-Control"[..], &b"max-age=3600"[..]).unwrap());
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
            "open-razor — lighting console for Razer keyboards on macOS\n\n\
             usage: open-razor [--port 7311] [--no-open] [--no-restore]\n\
             \x20      open-razor --probe\n\
             \x20      open-razor --install-login | --uninstall-login\n\
             \x20      open-razor --test\n\n\
             --probe            read firmware, device mode, brightness and the\n\
             \x20                  indicator LEDs straight from the board\n\
             --install-login    start at login and re-apply the saved look\n\
             --no-restore       do not re-apply the saved state on launch\n\n\
             Serves the console at http://127.0.0.1:7311. State and presets live in\n\
             ~/Library/Application Support/OpenRazor/state.json"
        );
        return;
    }

    let port = flag_value(&args, "--port")
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let open_browser = !args.iter().any(|a| a == "--no-open");
    let restore = !args.iter().any(|a| a == "--no-restore");

    let app = App::load();

    let store = match Server::http(("127.0.0.1", port)) {
        Ok(server) => server,
        Err(err) => {
            eprintln!("open-razor: cannot listen on 127.0.0.1:{port}: {err}");
            std::process::exit(1);
        }
    };

    let url = format!("http://127.0.0.1:{port}");
    println!("open-razor listening on {url}");
    if open_browser {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }

    if restore {
        let state = app.stored.lock().unwrap().state.clone();
        match app.apply(&state) {
            Ok(firmware) => println!("restored {} lighting (firmware {firmware})", state.effect),
            Err(err) => println!("note: {err}"),
        }
    }

    for mut request in store.incoming_requests() {
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
            (Method::Get, "/fonts/plex-mono.ttf") => {
                respond_bytes(request, FONT_MONO.to_vec(), "font/ttf")
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
        .join("dev.rakshak.open-razor.plist")
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
  <key>Label</key><string>dev.rakshak.open-razor</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>--no-open</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>StandardOutPath</key><string>/tmp/open-razor.log</string>
  <key>StandardErrorPath</key><string>/tmp/open-razor.log</string>
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
