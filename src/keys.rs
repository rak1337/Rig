//! System-wide key remapping and macro playback.
//!
//! Both need a CGEventTap, which needs Accessibility permission. The tap lives
//! on its own run loop thread; nothing here touches the HID path.

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
    pub steps: Vec<Step>,
}

#[derive(Default)]
struct Shared {
    remaps: Mutex<HashMap<u16, u16>>,
    recording: Mutex<Option<Vec<Step>>>,
    recording_start: Mutex<Option<Instant>>,
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

    /// Accessibility permission is required for the tap to be created at all.
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

    pub fn start_recording(&self) {
        let mut recording = self.shared.recording.lock().unwrap();
        *recording = Some(Vec::new());
        *self.shared.recording_start.lock().unwrap() = Some(Instant::now());
    }

    /// Stop recording and hand back what was captured.
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

    pub fn play(&self, steps: Vec<Step>) {
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
                // Tag it so the tap does not remap our own playback.
                CGEventSetIntegerValueField(event, SOURCE_USER_DATA_FIELD, SYNTHETIC_TAG);
                CGEventPost(HID_TAP, event);
                CFRelease(event);
            }
        });
    }
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

        // Ignore events we synthesised ourselves.
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
                    at_ms: at_ms.saturating_sub(if at_ms > 2 { 2 } else { 0 }),
                });
            }
        }

        if event_type != FLAGS_CHANGED {
            if let Some(target) = shared.remaps.lock().unwrap().get(&keycode).copied() {
                CGEventSetIntegerValueField(event, KEYCODE_FIELD, target as i64);
            }
        }

        event
    }
}
