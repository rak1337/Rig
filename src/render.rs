//! Host-rendered effects.
//!
//! The board only knows a handful of built-in matrix effects. Everything else
//! (fire, starlight, ripple, gradients, marquees, comets) is a frame rendered
//! here and pushed as a custom frame, which is exactly how Razer's own software
//! does it.

pub const ROWS: usize = 6;
pub const COLS: usize = 22;
pub const CELLS: usize = ROWS * COLS;

#[derive(Clone, Debug)]
pub struct Settings {
    pub active: bool,
    pub effect: String,
    pub c1: (u8, u8, u8),
    pub c2: (u8, u8, u8),
    /// 1..=4, the same scale the hardware effects use.
    pub speed: f32,
    /// 0..=1
    pub brightness: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            active: false,
            effect: String::new(),
            c1: (255, 255, 255),
            c2: (10, 132, 255),
            speed: 2.0,
            brightness: 1.0,
        }
    }
}

/// Effects this module renders. Everything else is a device command.
pub fn is_software(effect: &str) -> bool {
    matches!(
        effect,
        "fire" | "starlight" | "ripple" | "gradient" | "marquee" | "comet" | "type"
    )
}

fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(1_274_126_177));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65_535.0
}

fn scale(color: (f32, f32, f32), brightness: f32) -> (u8, u8, u8) {
    let c = |v: f32| (v * brightness).clamp(0.0, 255.0) as u8;
    (c(color.0), c(color.1), c(color.2))
}

fn rgb((r, g, b): (u8, u8, u8)) -> (f32, f32, f32) {
    (r as f32, g as f32, b as f32)
}

fn mix(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

fn mul(color: (f32, f32, f32), k: f32) -> (f32, f32, f32) {
    let k = k.clamp(0.0, 1.0);
    (color.0 * k, color.1 * k, color.2 * k)
}

/// One frame for the given time, in seconds.
pub fn frame(settings: &Settings, time: f32, presses: &[(usize, f32)]) -> Vec<(u8, u8, u8)> {
    let c1 = rgb(settings.c1);
    let c2 = rgb(settings.c2);
    let speed = settings.speed.max(0.5);
    let mut out = vec![(0, 0, 0); CELLS];

    for row in 0..ROWS {
        for col in 0..COLS {
            let x = col as f32 / (COLS - 1) as f32;
            let y = row as f32 / (ROWS - 1) as f32;
            let index = row * COLS + col;

            let color = match settings.effect.as_str() {
                // A flame that licks upward: per-cell noise plus a slow roll.
                "fire" => {
                    let flicker = hash(col as u32, row as u32, (time * 9.0 * speed) as u32);
                    let roll = ((time * 1.6 * speed) + row as f32 * 0.55 + col as f32 * 0.21).sin();
                    let heat = (flicker * 0.65 + (roll * 0.5 + 0.5) * 0.35)
                        * (1.0 - y * 0.45)
                        * (1.0 - y * 0.45);
                    let flame = (255.0, 90.0 + 120.0 * heat, 20.0 * heat);
                    mul(flame, heat * 1.15)
                }

                // Sparse, slowly twinkling points.
                "starlight" => {
                    let bucket = (time * 2.2 * speed) as u32;
                    let spark = hash(col as u32, row as u32, bucket);
                    let intensity = if spark > 0.94 {
                        (spark - 0.94) / 0.06
                    } else {
                        0.03
                    };
                    mul(c1, intensity)
                }

                // Rings travelling out from the middle of the board.
                "ripple" => {
                    let dx = col as f32 - (COLS as f32 - 1.0) / 2.0;
                    let dy = (row as f32 - (ROWS as f32 - 1.0) / 2.0) * 1.7;
                    let distance = (dx * dx + dy * dy).sqrt();
                    let wave = (distance * 0.85 - time * 3.4 * speed).sin();
                    let intensity = ((wave * 0.5 + 0.5) as f32).powf(2.2);
                    mul(c1, 0.06 + intensity * 0.94)
                }

                // Two colours drifting across the board.
                "gradient" => {
                    let drift = (time * 0.12 * speed) % 1.0;
                    let position = (x + drift) % 1.0;
                    // Mirror so the wrap is not a hard seam.
                    let t = if position < 0.5 { position * 2.0 } else { (1.0 - position) * 2.0 };
                    mix(c1, c2, t)
                }

                // A soft band sweeping left to right.
                "marquee" => {
                    let head = ((time * 0.32 * speed) % 1.4) - 0.2;
                    let distance = (x - head).abs();
                    let intensity = (1.0 - distance * 3.2).max(0.0);
                    let intensity = intensity * intensity * (3.0 - 2.0 * intensity);
                    mix(mul(c1, 0.05), c1, intensity)
                }

                // A head with a decaying tail.
                "comet" => {
                    let head = ((time * 0.42 * speed) % 1.3) - 0.15;
                    let behind = head - x;
                    let trail = if behind >= 0.0 {
                        (-behind * 5.0).exp()
                    } else {
                        0.0
                    };
                    let core = if (x - head).abs() < 0.06 { 1.0 } else { 0.0 };
                    mix(mul(c1, 0.03), c1, trail.max(core))
                }

                // Rings that start where a key was actually pressed.
                "type" => {
                    let mut intensity: f32 = 0.05;
                    for (cell, age) in presses {
                        let (press_row, press_col) = (cell / COLS, cell % COLS);
                        let dx = col as f32 - press_col as f32;
                        let dy = (row as f32 - press_row as f32) * 1.6;
                        let distance = (dx * dx + dy * dy).sqrt();
                        let radius = age * 9.0;
                        let ring = (-((distance - radius).abs()) * 1.1).exp();
                        let fade = (1.0 - age / 0.9).max(0.0);
                        intensity = intensity.max(ring * fade);
                    }
                    mul(c1, intensity)
                }

                _ => (0.0, 0.0, 0.0),
            };

            out[index] = scale(color, settings.brightness);
        }
    }

    out
}
