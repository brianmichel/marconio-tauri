use std::sync::OnceLock;

const SIZE: u32 = 44;
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;
const SCALE: usize = 3;
type IconRgba = (Vec<u8>, u32, u32);

static IDLE_ICON: OnceLock<IconRgba> = OnceLock::new();
static PRESET_ICONS: OnceLock<[IconRgba; 6]> = OnceLock::new();

/// 5-wide, 7-tall bitmap glyphs for digits 1–6.
/// Each byte's low 5 bits encode pixel columns left-to-right.
const DIGITS: [[u8; GLYPH_H]; 6] = [
    // 1
    [
        0b00100,
        0b01100,
        0b00100,
        0b00100,
        0b00100,
        0b00100,
        0b01110,
    ],
    // 2
    [
        0b01110,
        0b10001,
        0b00001,
        0b00110,
        0b01000,
        0b10000,
        0b11111,
    ],
    // 3
    [
        0b01110,
        0b10001,
        0b00001,
        0b00110,
        0b00001,
        0b10001,
        0b01110,
    ],
    // 4
    [
        0b00010,
        0b00110,
        0b01010,
        0b10010,
        0b11111,
        0b00010,
        0b00010,
    ],
    // 5
    [
        0b11111,
        0b10000,
        0b11110,
        0b00001,
        0b00001,
        0b10001,
        0b01110,
    ],
    // 6
    [
        0b01110,
        0b10001,
        0b10000,
        0b11110,
        0b10001,
        0b10001,
        0b01110,
    ],
];

/// Render a 44x44 RGBA tray icon with a pause symbol inside the rounded rectangle.
///
/// Used as the idle/stopped placeholder.
pub fn render_idle_icon() -> (Vec<u8>, u32, u32) {
    let s = SIZE as usize;
    let mut rgba = vec![0u8; s * s * 4];
    let (r, g, b) = icon_color();
    draw_rounded_rect(&mut rgba, s, r, g, b);

    // Two vertical bars, centered in the icon.
    let bar_w = 4_usize;
    let bar_h = 16_usize;
    let gap = 6_usize;
    let total_w = bar_w * 2 + gap;
    let ox = (s - total_w) / 2;
    let oy = (s - bar_h) / 2;

    for dy in 0..bar_h {
        for dx in 0..bar_w {
            set_pixel(&mut rgba, s, ox + dx, oy + dy, r, g, b, 255);
            set_pixel(&mut rgba, s, ox + bar_w + gap + dx, oy + dy, r, g, b, 255);
        }
    }

    (rgba, SIZE, SIZE)
}

/// Returns a cached idle icon bitmap.
pub fn cached_idle_icon() -> &'static IconRgba {
    IDLE_ICON.get_or_init(render_idle_icon)
}

/// Render a 44x44 RGBA tray icon displaying the given preset number (1–6).
///
/// Returns `(rgba_bytes, width, height)`.
pub fn render_preset_icon(preset: u8) -> (Vec<u8>, u32, u32) {
    let s = SIZE as usize;
    let mut rgba = vec![0u8; s * s * 4];

    let (r, g, b) = icon_color();

    draw_rounded_rect(&mut rgba, s, r, g, b);

    let digit_idx = (preset.saturating_sub(1) as usize).min(5);
    let glyph = &DIGITS[digit_idx];

    let gw = GLYPH_W * SCALE;
    let gh = GLYPH_H * SCALE;
    let ox = (s - gw) / 2;
    let oy = (s - gh) / 2;

    for (row, &bits) in glyph.iter().enumerate() {
        for col in 0..GLYPH_W {
            if bits & (1 << (4 - col)) != 0 {
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        let px = ox + col * SCALE + dx;
                        let py = oy + row * SCALE + dy;
                        set_pixel(&mut rgba, s, px, py, r, g, b, 255);
                    }
                }
            }
        }
    }

    (rgba, SIZE, SIZE)
}

/// Returns a cached preset icon bitmap.
pub fn cached_preset_icon(preset: u8) -> &'static IconRgba {
    let digit_idx = (preset.saturating_sub(1) as usize).min(5);
    let icons = PRESET_ICONS.get_or_init(|| {
        [
            render_preset_icon(1),
            render_preset_icon(2),
            render_preset_icon(3),
            render_preset_icon(4),
            render_preset_icon(5),
            render_preset_icon(6),
        ]
    });
    &icons[digit_idx]
}

/// Returns the foreground color for the tray icon.
///
/// macOS template icons use black — the OS handles light/dark adaptation.
/// Windows system-tray icons need white to be visible on the dark taskbar.
fn icon_color() -> (u8, u8, u8) {
    #[cfg(target_os = "windows")]
    {
        (255, 255, 255)
    }
    #[cfg(not(target_os = "windows"))]
    {
        (0, 0, 0)
    }
}

/// Draw an anti-aliased rounded-rectangle outline using a signed distance field.
fn draw_rounded_rect(rgba: &mut [u8], s: usize, r: u8, g: u8, b: u8) {
    let inset = 4.0_f32;
    let radius = 6.0_f32;
    let stroke = 2.0_f32;

    let left = inset + radius;
    let right = s as f32 - inset - radius;
    let top = inset + radius;
    let bottom = s as f32 - inset - radius;

    for y in 0..s {
        for x in 0..s {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let cx = fx.clamp(left, right);
            let cy = fy.clamp(top, bottom);

            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt() - radius;

            // Distance from the center of the stroke band (at dist = -stroke/2).
            let half = stroke / 2.0;
            let center_dist = dist + half;
            let alpha = (half - center_dist.abs() + 0.5).clamp(0.0, 1.0);

            if alpha > 0.0 {
                let a = (alpha * 255.0) as u8;
                set_pixel(rgba, s, x, y, r, g, b, a);
            }
        }
    }
}

fn set_pixel(rgba: &mut [u8], stride: usize, x: usize, y: usize, r: u8, g: u8, b: u8, a: u8) {
    let offset = (y * stride + x) * 4;
    let ea = rgba[offset + 3] as f32 / 255.0;
    let na = a as f32 / 255.0;
    let out_a = na + ea * (1.0 - na);

    if out_a > 0.0 {
        let inv = 1.0 / out_a;
        rgba[offset] = ((r as f32 * na + rgba[offset] as f32 * ea * (1.0 - na)) * inv) as u8;
        rgba[offset + 1] =
            ((g as f32 * na + rgba[offset + 1] as f32 * ea * (1.0 - na)) * inv) as u8;
        rgba[offset + 2] =
            ((b as f32 * na + rgba[offset + 2] as f32 * ea * (1.0 - na)) * inv) as u8;
        rgba[offset + 3] = (out_a * 255.0) as u8;
    }
}
