//! Razer vendor protocol: report construction and effect definitions.
//!
//! Layout of the 90-byte report (see OpenRazer `razercommon.h`):
//!
//! ```text
//! 0      status (0x00 new command, 0x02 successful)
//! 1      transaction id
//! 2-3    remaining packets (big endian)
//! 4      protocol type
//! 5      data size
//! 6      command class
//! 7      command id
//! 8-87   arguments
//! 88     CRC: XOR of bytes 2..87
//! 89     reserved
//! ```

use crate::hid::REPORT_LEN;

pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_OK: u8 = 0x02;

// Matrix effect ids.
pub const FX_OFF: u8 = 0x00;
pub const FX_CUSTOM: u8 = 0x05;

// LED ids and storage selectors.
const VARSTORE: u8 = 0x01;
const NOSTORE: u8 = 0x00;
const LED_BACKLIGHT: u8 = 0x05;
const MACRO_LED: u8 = 0x07;
const GAME_LED: u8 = 0x08;

// Command classes.
const CLASS_DEVICE: u8 = 0x00;
const CLASS_LED: u8 = 0x03;

pub const MATRIX_ROWS: usize = 6;
pub const MATRIX_COLS: usize = 22;

/// Assemble a report with a valid checksum.
pub fn build(class: u8, command: u8, data_size: u8, args: &[u8]) -> [u8; REPORT_LEN] {
    let mut report = [0u8; REPORT_LEN];
    report[0] = 0x00;
    report[1] = 0xFF;
    report[5] = data_size;
    report[6] = class;
    report[7] = command;
    for (index, byte) in args.iter().take(80).enumerate() {
        report[8 + index] = *byte;
    }
    let mut crc = 0u8;
    for byte in &report[2..88] {
        crc ^= byte;
    }
    report[88] = crc;
    report
}

fn effect(effect_id: u8, args: &[u8]) -> [u8; REPORT_LEN] {
    let mut payload = Vec::with_capacity(args.len() + 1);
    payload.push(effect_id);
    payload.extend_from_slice(args);
    build(CLASS_LED, 0x0A, payload.len() as u8, &payload)
}

pub fn off() -> [u8; REPORT_LEN] {
    effect(FX_OFF, &[])
}

/// Switch the matrix into custom-frame mode. Frames are pushed with
/// [`custom_frame_row`] afterwards.
pub fn custom_effect() -> [u8; REPORT_LEN] {
    effect(FX_CUSTOM, &[VARSTORE])
}

/// One row of a custom frame: 22 RGB triplets for columns 0..=21.
pub fn custom_frame_row(row: usize, colors: &[(u8, u8, u8)]) -> [u8; REPORT_LEN] {
    let mut args = Vec::with_capacity(4 + MATRIX_COLS * 3);
    args.push(0xFF); // frame id
    args.push(row as u8);
    args.push(0x00); // start column
    args.push((MATRIX_COLS - 1) as u8); // stop column
    for (r, g, b) in colors.iter().take(MATRIX_COLS) {
        args.extend_from_slice(&[*r, *g, *b]);
    }
    build(CLASS_LED, 0x0B, args.len() as u8, &args)
}

pub fn brightness(percent: u8) -> [u8; REPORT_LEN] {
    let percent = percent.min(100) as u32;
    let level = ((percent * 255 + 50) / 100) as u8;
    build(
        CLASS_LED,
        0x03,
        0x03,
        &[VARSTORE, LED_BACKLIGHT, level],
    )
}

/// Toggle a single indicator LED (game mode, macro recording).
pub fn indicator(enabled: bool, game_mode: bool) -> [u8; REPORT_LEN] {
    let (storage, led) = if game_mode {
        (NOSTORE, GAME_LED)
    } else {
        (VARSTORE, MACRO_LED)
    };
    build(CLASS_LED, 0x00, 0x03, &[storage, led, if enabled { 0x01 } else { 0x00 }])
}

/// 0x03 hands LED control to software, 0x00 returns it to the firmware.
pub fn device_mode(mode: u8) -> [u8; REPORT_LEN] {
    build(CLASS_DEVICE, 0x04, 0x02, &[mode, 0x00])
}

pub fn firmware_version() -> [u8; REPORT_LEN] {
    build(CLASS_DEVICE, 0x81, 0x02, &[])
}

// --- reads: what the board currently has stored -------------------------

pub const LED_GAME: u8 = GAME_LED;
pub const LED_MACRO: u8 = MACRO_LED;

/// Which side owns the LEDs: 0x00 firmware, 0x03 software.
pub fn get_device_mode() -> [u8; REPORT_LEN] {
    build(CLASS_DEVICE, 0x84, 0x02, &[])
}

pub fn get_led_state(led: u8) -> [u8; REPORT_LEN] {
    build(CLASS_LED, 0x80, 0x03, &[VARSTORE, led])
}

pub fn get_brightness() -> [u8; REPORT_LEN] {
    build(CLASS_LED, 0x83, 0x03, &[VARSTORE, LED_BACKLIGHT])
}
