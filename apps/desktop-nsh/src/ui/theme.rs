//! The product's palette and spacing: a restrained cinema look, dark by
//! design. Colors are static rather than following the system appearance —
//! a video app's chrome should not turn white under the video.

use nsh_core::{
    element::{FontWeight, Rgba, rgb, rgba},
    geometry::px,
    prelude::*,
};

/// The page behind everything.
pub const BG: Rgba = rgb(0x0a0c10);
/// The navigation column.
pub const SIDEBAR: Rgba = rgb(0x0d1015);
/// A card or a panel.
pub const SURFACE: Rgba = rgb(0x12161d);
/// A control on a card.
pub const RAISED: Rgba = rgb(0x171c24);
/// A hovered control.
pub const HOVER: Rgba = rgb(0x1d232d);
/// Borders of cards and controls.
pub const BORDER: Rgba = rgb(0x252c37);
/// Hairlines inside surfaces.
pub const LINE: Rgba = rgb(0x1b212b);
/// Primary text.
pub const TEXT: Rgba = rgb(0xf2f4f8);
/// Secondary text.
pub const MUTED: Rgba = rgb(0x9aa4b2);
/// Quiet text, placeholders, counters.
pub const FAINT: Rgba = rgb(0x66707e);
/// The warm accent: a lamp in a dark room.
pub const ACCENT: Rgba = rgb(0xe9b872);
/// A brighter accent for hover.
pub const ACCENT_BRIGHT: Rgba = rgb(0xf5ce8f);
/// Text on the accent fill.
pub const ON_ACCENT: Rgba = rgb(0x1b1307);
/// Danger and destructive actions.
pub const DANGER: Rgba = rgb(0xf07b7b);
/// Success and connected states.
pub const SUCCESS: Rgba = rgb(0x62d6a3);
/// Information.
pub const INFO: Rgba = rgb(0x86b4ff);
/// The demo mode's amber tint.
pub const DEMO_TINT: Rgba = rgba(0xe9b87222);
/// A danger tint behind text.
pub const DANGER_TINT: Rgba = rgba(0xf07b7b1f);
/// The accent tint behind text.
pub const ACCENT_TINT: Rgba = rgba(0xe9b8721f);
/// The success tint behind text.
pub const SUCCESS_TINT: Rgba = rgba(0x62d6a320);
/// Pure black for the video stage.
pub const STAGE: Rgba = rgb(0x000000);

/// `#rrggbb` at `alpha` (0..1).
pub const fn alpha(hex: u32, a: f32) -> Rgba {
    rgba(hex << 8 | (a * 255.0 + 0.5) as u32)
}

/// The background gradient pair a title's poster uses, chosen by a stable
/// hash of the title. These are title cards, not photographs.
pub fn poster_colors(title: &str) -> (Rgba, Rgba, Rgba) {
    let mut hash: u32 = 2166136261;
    for b in title.as_bytes() {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    const SETS: [(u32, u32, u32); 6] = [
        (0x24384a, 0x0e1720, 0x8fb7d6),
        (0x3a2a22, 0x160f0b, 0xe0a878),
        (0x213a34, 0x0b1512, 0x86c7b6),
        (0x2e2a3e, 0x110f18, 0xb3a8dd),
        (0x3b2a31, 0x150d11, 0xd9a0ad),
        (0x2b3324, 0x10130c, 0xb8c98e),
    ];
    let (a, b, ink) = SETS[(hash as usize) % SETS.len()];
    (rgb(a), rgb(b), rgb(ink))
}

pub const RADIUS_CARD: f32 = 14.0;
pub const RADIUS_CONTROL: f32 = 9.0;
pub const SIDEBAR_W: f32 = 224.0;
pub const TOPBAR_H: f32 = 64.0;

/// A page's title.
pub fn h1(value: impl Into<String>) -> Text {
    text(value.into())
        .text_size(px(27.0))
        .font_weight(FontWeight::BOLD)
        .text_color(TEXT)
}

/// A section's title.
pub fn h2(value: impl Into<String>) -> Text {
    text(value.into())
        .text_size(px(17.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(TEXT)
}

/// Secondary copy.
pub fn note(value: impl Into<String>) -> Text {
    text(value.into())
        .text_size(px(13.0))
        .text_color(MUTED)
        .line_height(nsh_core::LineHeight::Multiple(1.55))
}

/// A one-line label above a value.
pub fn overline(value: impl Into<String>) -> Text {
    text(value.into())
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(FAINT)
        .letter_spacing(px(0.8))
}
