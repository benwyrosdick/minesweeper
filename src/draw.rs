//! Drawing primitives for the classic look: bevels, pixel-art digits, LED counters,
//! the face button, mines and flags. Everything is procedural, so no asset files are needed.

use macroquad::prelude::*;

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

pub const SILVER: Color = rgb(192, 192, 192);
pub const SHADOW: Color = rgb(128, 128, 128);
pub const HILITE: Color = rgb(255, 255, 255);
pub const INK: Color = rgb(0, 0, 0);
pub const NAVY: Color = rgb(0, 0, 128);
pub const ALARM: Color = rgb(255, 0, 0);
pub const LED_ON: Color = rgb(255, 0, 0);
pub const LED_OFF: Color = rgb(44, 0, 0);
pub const SMILEY: Color = rgb(255, 255, 0);

/// The classic number palette, indexed by `n - 1`.
pub const NUMBER_COLORS: [Color; 8] = [
    rgb(0, 0, 255),     // 1 blue
    rgb(0, 128, 0),     // 2 green
    rgb(255, 0, 0),     // 3 red
    rgb(0, 0, 128),     // 4 navy
    rgb(128, 0, 0),     // 5 maroon
    rgb(0, 128, 128),   // 6 teal
    rgb(0, 0, 0),       // 7 black
    rgb(128, 128, 128), // 8 gray
];

/// A 3D edge: light top-left and dark bottom-right when raised, the reverse when sunken.
pub fn bevel(x: f32, y: f32, w: f32, h: f32, thickness: f32, raised: bool) {
    let (light, dark) = if raised { (HILITE, SHADOW) } else { (SHADOW, HILITE) };
    for i in 0..thickness as i32 {
        let i = i as f32;
        draw_rectangle(x, y + i, w - i, 1.0, light);
        draw_rectangle(x + i, y, 1.0, h - i, light);
        draw_rectangle(x + i + 1.0, y + h - 1.0 - i, w - i - 1.0, 1.0, dark);
        draw_rectangle(x + w - 1.0 - i, y + i + 1.0, 1.0, h - i - 1.0, dark);
    }
}

pub fn panel(x: f32, y: f32, w: f32, h: f32, thickness: f32, raised: bool) {
    draw_rectangle(x, y, w, h, SILVER);
    bevel(x, y, w, h, thickness, raised);
}

// ---------------------------------------------------------------- pixel glyphs

const DIGITS: [[&str; 11]; 8] = [
    [
        "...###...", "..####...", ".#####...", "...###...", "...###...", "...###...",
        "...###...", "...###...", "...###...", ".#######.", ".#######.",
    ],
    [
        ".#######.", "###...###", "......###", "......###", "....####.", "..####...",
        ".###.....", "###......", "###......", "#########", "#########",
    ],
    [
        ".#######.", "###...###", "......###", "......###", "...#####.", "...#####.",
        "......###", "......###", "###...###", "###...###", ".#######.",
    ],
    [
        "###...###", "###...###", "###...###", "###...###", "###...###", "#########",
        "#########", "......###", "......###", "......###", "......###",
    ],
    [
        "#########", "###......", "###......", "###......", "########.", "......###",
        "......###", "......###", "###...###", "###...###", ".#######.",
    ],
    [
        ".#######.", "###...###", "###......", "###......", "########.", "###...###",
        "###...###", "###...###", "###...###", "###...###", ".#######.",
    ],
    [
        "#########", "#########", "......###", "......###", ".....###.", "....###..",
        "...###...", "...###...", "...###...", "...###...", "...###...",
    ],
    [
        ".#######.", "###...###", "###...###", "###...###", ".#######.", "###...###",
        "###...###", "###...###", "###...###", "###...###", ".#######.",
    ],
];

const QUESTION: [&str; 11] = [
    ".#######.", "###...###", "###...###", "......###", "....####.", "...###...",
    "...###...", "...###...", ".........", "...###...", "...###...",
];

fn glyph_centered(rows: &[&str], cx: f32, cy: f32, px: f32, color: Color) {
    let w = rows[0].len() as f32 * px;
    let h = rows.len() as f32 * px;
    let (x0, y0) = ((cx - w / 2.0).round(), (cy - h / 2.0).round());
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                draw_rectangle(x0 + c as f32 * px, y0 + r as f32 * px, px, px, color);
            }
        }
    }
}

fn glyph_px(size: f32) -> f32 {
    (size / 16.0).floor().max(1.0)
}

pub fn number(n: u8, x: f32, y: f32, size: f32) {
    let i = (n as usize).clamp(1, 8) - 1;
    glyph_centered(&DIGITS[i], x + size / 2.0 + 1.0, y + size / 2.0 + 1.0, glyph_px(size), NUMBER_COLORS[i]);
}

pub fn question(x: f32, y: f32, size: f32) {
    glyph_centered(&QUESTION, x + size / 2.0, y + size / 2.0, glyph_px(size), INK);
}

// ---------------------------------------------------------------- mine & flag

pub fn mine(x: f32, y: f32, s: f32) {
    let (cx, cy) = (x + s / 2.0 + 1.0, y + s / 2.0 + 1.0);
    let r = s * 0.27;
    let t = (s / 14.0).max(1.5);
    let l = s * 0.36;
    let d = l * 0.72;
    draw_line(cx - l, cy, cx + l, cy, t, INK);
    draw_line(cx, cy - l, cx, cy + l, t, INK);
    draw_line(cx - d, cy - d, cx + d, cy + d, t, INK);
    draw_line(cx - d, cy + d, cx + d, cy - d, t, INK);
    draw_circle(cx, cy, r, INK);
    let hs = (s * 0.1).round();
    draw_rectangle((cx - r * 0.55).round(), (cy - r * 0.55).round(), hs, hs, HILITE);
}

pub fn wrong_flag(x: f32, y: f32, s: f32) {
    mine(x, y, s);
    let (a, b) = (s * 0.18, s * 0.86);
    let t = (s / 12.0).max(2.0);
    draw_line(x + a, y + a, x + b, y + b, t, ALARM);
    draw_line(x + a, y + b, x + b, y + a, t, ALARM);
}

pub fn flag(x: f32, y: f32, s: f32) {
    let u = s / 16.0;
    draw_triangle(
        vec2(x + 9.0 * u, y + 2.5 * u),
        vec2(x + 9.0 * u, y + 8.5 * u),
        vec2(x + 3.0 * u, y + 5.5 * u),
        ALARM,
    );
    draw_rectangle(x + 8.5 * u, y + 2.5 * u, 1.25 * u, 9.0 * u, INK);
    draw_rectangle(x + 6.0 * u, y + 10.5 * u, 6.0 * u, 1.25 * u, INK);
    draw_rectangle(x + 4.0 * u, y + 11.5 * u, 10.0 * u, 1.75 * u, INK);
}

// ---------------------------------------------------------------- LED counter

pub const LED_W: f32 = 82.0;
pub const LED_H: f32 = 50.0;
const LED_DIGIT_W: f32 = 26.0;

// Segment bits: a=1 b=2 c=4 d=8 e=16 f=32 g=64. Index 10 is a minus sign.
const SEGMENTS: [u8; 11] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F, 0x40];

/// A three-digit red seven-segment display, clamped to -99..=999.
pub fn led(value: i32, x: f32, y: f32) {
    bevel(x, y, LED_W, LED_H, 1.0, false);
    draw_rectangle(x + 1.0, y + 1.0, LED_W - 2.0, LED_H - 2.0, INK);

    let v = value.clamp(-99, 999);
    let digits: [usize; 3] = if v < 0 {
        let a = (-v) as usize;
        [10, a / 10, a % 10]
    } else {
        let a = v as usize;
        [a / 100, a / 10 % 10, a % 10]
    };
    for (i, &d) in digits.iter().enumerate() {
        led_digit(SEGMENTS[d], x + 2.0 + i as f32 * LED_DIGIT_W, y + 2.0);
    }
}

fn led_digit(bits: u8, dx: f32, dy: f32) {
    let (x0, y0, w, h, t) = (dx + 3.0, dy + 3.0, 20.0, 40.0, 4.0);
    let half = h / 2.0;
    let vlen = half - 1.5 * t - 2.0;
    let segs = [
        (x0 + t, y0, w - 2.0 * t, t),                              // a
        (x0 + w - t, y0 + t + 1.0, t, vlen),                       // b
        (x0 + w - t, y0 + half + t / 2.0 + 1.0, t, vlen),          // c
        (x0 + t, y0 + h - t, w - 2.0 * t, t),                      // d
        (x0, y0 + half + t / 2.0 + 1.0, t, vlen),                  // e
        (x0, y0 + t + 1.0, t, vlen),                               // f
        (x0 + t, y0 + half - t / 2.0, w - 2.0 * t, t),             // g
    ];
    for (i, &(sx, sy, sw, sh)) in segs.iter().enumerate() {
        let lit = bits & (1 << i) != 0;
        draw_rectangle(sx, sy, sw, sh, if lit { LED_ON } else { LED_OFF });
    }
}

// ---------------------------------------------------------------- face button

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Smile,
    Surprised,
    Dead,
    Cool,
}

pub const FACE_SIZE: f32 = 52.0;

pub fn face_button(x: f32, y: f32, face: Face, pressed: bool) {
    let s = FACE_SIZE;
    draw_rectangle(x, y, s, s, SHADOW);
    let (ix, iy, is) = (x + 1.0, y + 1.0, s - 2.0);
    draw_rectangle(ix, iy, is, is, SILVER);
    if pressed {
        draw_rectangle(ix, iy, is, 2.0, SHADOW);
        draw_rectangle(ix, iy, 2.0, is, SHADOW);
    } else {
        bevel(ix, iy, is, is, 4.0, true);
    }

    let off = if pressed { 2.0 } else { 0.0 };
    let (cx, cy) = (x + s / 2.0 + off, y + s / 2.0 + off);
    let r = 17.0;
    draw_circle(cx, cy, r + 1.5, INK);
    draw_circle(cx, cy, r, SMILEY);

    let eyes = || {
        draw_rectangle(cx - 7.0, cy - 7.0, 3.0, 4.0, INK);
        draw_rectangle(cx + 4.0, cy - 7.0, 3.0, 4.0, INK);
    };
    let smile = || arc(cx, cy + 1.0, 9.0, 30.0, 150.0, 2.5);

    match face {
        Face::Smile => {
            eyes();
            smile();
        }
        Face::Surprised => {
            eyes();
            draw_circle(cx, cy + 7.0, 4.5, INK);
            draw_circle(cx, cy + 7.0, 2.3, SMILEY);
        }
        Face::Dead => {
            for ex in [cx - 6.0, cx + 6.0] {
                draw_line(ex - 3.0, cy - 8.0, ex + 3.0, cy - 2.0, 2.0, INK);
                draw_line(ex - 3.0, cy - 2.0, ex + 3.0, cy - 8.0, 2.0, INK);
            }
            arc(cx, cy + 14.0, 8.0, 215.0, 325.0, 2.5);
        }
        Face::Cool => {
            draw_rectangle(cx - 11.0, cy - 8.0, 9.0, 6.0, INK);
            draw_rectangle(cx + 2.0, cy - 8.0, 9.0, 6.0, INK);
            draw_line(cx - 11.0, cy - 7.0, cx + 11.0, cy - 7.0, 2.0, INK);
            draw_line(cx - 11.0, cy - 7.0, cx - 16.0, cy - 10.0, 2.0, INK);
            draw_line(cx + 11.0, cy - 7.0, cx + 16.0, cy - 10.0, 2.0, INK);
            smile();
        }
    }
}

fn arc(cx: f32, cy: f32, r: f32, start_deg: f32, end_deg: f32, thickness: f32) {
    let steps = 12;
    let point = |i: i32| {
        let a = (start_deg + (end_deg - start_deg) * i as f32 / steps as f32).to_radians();
        vec2(cx + r * a.cos(), cy + r * a.sin())
    };
    for i in 0..steps {
        let (p, q) = (point(i), point(i + 1));
        draw_line(p.x, p.y, q.x, q.y, thickness, INK);
    }
}

pub fn checkmark(x: f32, y: f32, color: Color) {
    draw_line(x, y + 5.0, x + 3.0, y + 8.0, 2.0, color);
    draw_line(x + 3.0, y + 8.0, x + 9.0, y + 1.0, 2.0, color);
}
