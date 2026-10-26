//! System idle time, for the backlight timeout.

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
}

const HID_SYSTEM_STATE: i32 = 1;
const ANY_INPUT_EVENT: u32 = 0xFFFF_FFFF;

/// Seconds since the last keyboard, mouse or trackpad event.
pub fn seconds_since_input() -> f64 {
    unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT_EVENT) }
}
