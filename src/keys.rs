//! System-wide key remapping, macro playback and keypress feed.
//!
//! All of it runs off one CGEventTap, which needs Accessibility permission. The
//! tap lives on its own run loop thread; nothing here touches the HID path.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;

const KEY_DOWN: u32 = 10;
const KEY_UP: u32 = 11;
const FLAGS_CHANGED: u32 = 12;
const KEYCODE_FIELD: u32 = 9;
const SOURCE_USER_DATA_FIELD: u32 = 42;
const SYNTHETIC_TAG: i64 = 0x5249_4721; // "RIG!"
const HID_TAP: u32 = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventCreateKeyboardEvent(source: *mut c_void, keycode: u16, down: bool) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopAddSource(run_loop: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopRun();
    fn CFRelease(cf: *const c_void);
    static kCFRunLoopCommonModes: *const c_void;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Step {
    pub key: u16,
    pub down: bool,
    pub at_ms: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Macro {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub trigger: Option<u16>,
    pub steps: Vec<Step>,
}

#[derive(Default)]
struct Shared {
    remaps: Mutex<HashMap<u16, u16>>,
    recording: Mutex<Option<Vec<Step>>>,
    recording_start: Mutex<Option<Instant>>,
    /// (keycode, when) for every real keypress, drained by the renderer.
    presses: Mutex<Vec<(u16, Instant)>>,
    triggers: Mutex<HashMap<u16, Vec<Step>>>,
    hyper_key: Mutex<Option<u16>>,
    layer2: Mutex<HashMap<u16, u16>>,
    held: Mutex<std::collections::HashSet<u16>>,
}

#[derive(Clone)]
pub struct Engine {
    shared: Arc<Shared>,
    running: Arc<Mutex<bool>>,
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            shared: Arc::new(Shared::default()),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn trusted(&self) -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn start(&self) {
        let mut running = self.running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || unsafe {
            let mask = (1u64 << KEY_DOWN) | (1u64 << KEY_UP) | (1u64 << FLAGS_CHANGED);
            let context = Arc::into_raw(shared) as *mut c_void;
            let tap = CGEventTapCreate(HID_TAP, 0, 0, mask, callback, context);
            if tap.is_null() {
                return; // no permission; the UI explains why
            }
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            CFRunLoopRun();
        });
    }

    pub fn set_remaps(&self, remaps: &[(u16, u16)]) {
        let mut map = self.shared.remaps.lock().unwrap();
        map.clear();
        for (from, to) in remaps {
            map.insert(*from, *to);
        }
    }

    pub fn set_hypershift(&self, key: Option<u16>, layer2: &[(u16, u16)]) {
        *self.shared.hyper_key.lock().unwrap() = key;
        let mut map = self.shared.layer2.lock().unwrap();
        map.clear();
        for (from, to) in layer2 {
            map.insert(*from, *to);
        }
    }

    pub fn set_triggers(&self, triggers: &[(u16, Vec<Step>)]) {
        let mut map = self.shared.triggers.lock().unwrap();
        map.clear();
        for (key, steps) in triggers {
            map.insert(*key, steps.clone());
        }
    }

    pub fn start_recording(&self) {
        *self.shared.recording.lock().unwrap() = Some(Vec::new());
        *self.shared.recording_start.lock().unwrap() = Some(Instant::now());
    }

    pub fn stop_recording(&self) -> Vec<Step> {
        let steps = self.shared.recording.lock().unwrap().take().unwrap_or_default();
        *self.shared.recording_start.lock().unwrap() = None;
        steps
    }

    pub fn is_recording(&self) -> bool {
        self.shared.recording.lock().unwrap().is_some()
    }

    pub fn recorded_len(&self) -> usize {
        self.shared
            .recording
            .lock()
            .unwrap()
            .as_ref()
            .map(|steps| steps.len())
            .unwrap_or(0)
    }

    /// Keypresses since the last call, with their age in seconds.
    pub fn drain_presses(&self) -> Vec<(u16, f32)> {
        let now = Instant::now();
        let mut presses = self.shared.presses.lock().unwrap();
        let drained = presses
            .iter()
            .map(|(key, at)| (*key, now.duration_since(*at).as_secs_f32()))
            .filter(|(_, age)| *age < 1.2)
            .collect();
        presses.clear();
        drained
    }

    pub fn play(&self, steps: Vec<Step>) {
        play_steps(steps);
    }
}

fn play_steps(steps: Vec<Step>) {
    std::thread::spawn(move || unsafe {
        let start = Instant::now();
        for step in steps {
            let target = Duration::from_millis(step.at_ms);
            let elapsed = start.elapsed();
            if target > elapsed {
                std::thread::sleep(target - elapsed);
            }
            let event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), step.key, step.down);
            if event.is_null() {
                continue;
            }
            // Tag it so the tap ignores its own playback.
            CGEventSetIntegerValueField(event, SOURCE_USER_DATA_FIELD, SYNTHETIC_TAG);
            CGEventPost(HID_TAP, event);
            CFRelease(event);
        }
    });
}

extern "C" fn callback(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    unsafe {
        if event.is_null() || user_info.is_null() {
            return event;
        }
        let shared = &*(user_info as *const Shared);

        if CGEventGetIntegerValueField(event, SOURCE_USER_DATA_FIELD) == SYNTHETIC_TAG {
            return event;
        }

        let keycode = CGEventGetIntegerValueField(event, KEYCODE_FIELD) as u16;

        if event_type == KEY_DOWN || event_type == KEY_UP {
            if let Some(buffer) = shared.recording.lock().unwrap().as_mut() {
                let at_ms = shared
                    .recording_start
                    .lock()
                    .unwrap()
                    .map(|start| start.elapsed().as_millis() as u64)
                    .unwrap_or(0);
                buffer.push(Step {
                    key: keycode,
                    down: event_type == KEY_DOWN,
                    at_ms,
                });
            }
        }

        if event_type == KEY_DOWN {
            shared.presses.lock().unwrap().push((keycode, Instant::now()));
            let triggered = shared.triggers.lock().unwrap().get(&keycode).cloned();
            if let Some(steps) = triggered {
                play_steps(steps);
            }
        }

        match event_type {
            KEY_DOWN => {
                shared.held.lock().unwrap().insert(keycode);
            }
            KEY_UP => {
                shared.held.lock().unwrap().remove(&keycode);
            }
            _ => {}
        }

        if event_type != FLAGS_CHANGED {
            let hyper = *shared.hyper_key.lock().unwrap();
            let hypershifted = match hyper {
                Some(key) => shared.held.lock().unwrap().contains(&key),
                None => false,
            };
            let target = if hypershifted {
                shared.layer2.lock().unwrap().get(&keycode).copied()
            } else {
                shared.remaps.lock().unwrap().get(&keycode).copied()
            };
            if let Some(target) = target {
                CGEventSetIntegerValueField(event, KEYCODE_FIELD, target as i64);
            }
        }

        event
    }
}

/// Keycode to LED cell (row * 22 + column) for the BlackWidow TE Chroma.
pub fn cell_for_keycode(key: u16) -> Option<usize> {
    let cell = match key {
        0 => 68,   // A
        1 => 69,   // S
        2 => 70,   // D
        3 => 71,   // F
        4 => 73,   // H
        5 => 72,   // G
        6 => 91,   // Z
        7 => 92,   // X
        8 => 93,   // C
        9 => 94,   // V
        11 => 95,  // B
        12 => 46,  // Q
        13 => 47,  // W
        14 => 48,  // E
        15 => 49,  // R
        16 => 51,  // Y
        17 => 50,  // T
        18 => 24,  // 1
        19 => 25,  // 2
        20 => 26,  // 3
        21 => 27,  // 4
        22 => 29,  // 6
        23 => 28,  // 5
        24 => 35,  // =
        25 => 32,  // 9
        26 => 30,  // 7
        27 => 34,  // -
        28 => 31,  // 8
        29 => 33,  // 0
        30 => 57,  // ]
        31 => 54,  // O
        32 => 52,  // U
        33 => 56,  // [
        34 => 53,  // I
        35 => 55,  // P
        36 => 80,  // Return
        37 => 76,  // L
        38 => 74,  // J
        39 => 78,  // '
        40 => 75,  // K
        41 => 77,  // ;
        42 => 58,  // backslash
        43 => 98,  // ,
        44 => 100, // /
        45 => 96,  // N
        46 => 97,  // M
        47 => 99,  // .
        48 => 45,  // Tab
        49 => 117, // Space
        50 => 23,  // `
        51 => 36,  // Delete
        53 => 1,   // Escape
        55 => 112, // Command
        56 => 89,  // Shift
        57 => 67,  // Caps
        58 => 113, // Option
        59 => 111, // Control
        60 => 102, // Right shift
        61 => 121, // Right option
        62 => 124, // Right control
        96 => 7,   // F5
        97 => 8,   // F6
        98 => 9,   // F7
        99 => 5,   // F3
        100 => 10, // F8
        101 => 11, // F9
        103 => 13, // F11
        109 => 12, // F10
        111 => 14, // F12
        115 => 38, // Home
        116 => 39, // PageUp
        117 => 59, // Forward delete
        118 => 6,  // F4
        119 => 60, // End
        120 => 4,  // F2
        121 => 61, // PageDown
        122 => 3,  // F1
        123 => 125, // Left
        124 => 127, // Right
        125 => 126, // Down
        126 => 104, // Up
        _ => return None,
    };
    Some(cell)
}
