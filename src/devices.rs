//! Known devices and the layout each one should be drawn with.
//!
//! The console asks for a layout id instead of assuming one keyboard, so a board
//! we have not seen yet still renders — just generically.

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct DeviceInfo {
    pub pid: u16,
    pub name: &'static str,
    /// Layout id the UI knows how to draw.
    pub layout: &'static str,
    pub rows: usize,
    pub cols: usize,
    /// Whether per-key painting can address this board's matrix.
    pub per_key: bool,
    /// Whether to draw the logo plate on the front edge.
    pub logo: bool,
}

struct Known {
    pid: u16,
    name: &'static str,
    layout: &'static str,
    rows: usize,
    cols: usize,
    per_key: bool,
    logo: bool,
}

const KNOWN: &[Known] = &[
    Known {
        pid: 0x0209,
        name: "BlackWidow Tournament Edition Chroma",
        layout: "tkl",
        rows: 6,
        cols: 22,
        per_key: true,
        logo: true,
    },
    Known {
        pid: 0x0203,
        name: "BlackWidow Chroma",
        layout: "generic",
        rows: 6,
        cols: 22,
        per_key: false,
        logo: true,
    },
    Known {
        pid: 0x0221,
        name: "BlackWidow Chroma V2",
        layout: "generic",
        rows: 6,
        cols: 22,
        per_key: false,
        logo: true,
    },
];

pub const DEFAULT_PID: u16 = 0x0209;

pub fn info(pid: u16) -> DeviceInfo {
    match KNOWN.iter().find(|entry| entry.pid == pid) {
        Some(entry) => DeviceInfo {
            pid: entry.pid,
            name: entry.name,
            layout: entry.layout,
            rows: entry.rows,
            cols: entry.cols,
            per_key: entry.per_key,
            logo: entry.logo,
        },
        None => DeviceInfo {
            pid,
            name: "Unknown Razer keyboard",
            layout: "generic",
            rows: 6,
            cols: 22,
            per_key: false,
            logo: false,
        },
    }
}
