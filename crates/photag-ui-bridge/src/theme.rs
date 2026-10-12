//! PhoTag's shared GPUI design tokens, ported from `vendor/photag-source/src/theme.rs`.
//!
//! Colors are packed `0xRRGGBB` values for direct use with `gpui_kit::rgb`.
//! The source's egui drawing helpers are intentionally represented as data so
//! feature crates can paint with GPUI while sharing the original palette.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const SURFACE_BASE: u32 = 0x191919;
pub const SURFACE_PANEL: u32 = 0x252525;
pub const SURFACE_ELEVATED: u32 = 0x2e2e2e;
pub const SURFACE_MODAL: u32 = 0x2b2b2b;
pub const SURFACE_FAINT: u32 = 0x1f1f1f;
pub const TEXT_PRIMARY: u32 = 0xebebeb;
pub const TEXT_SECONDARY: u32 = 0xa0a0a0;
pub const TEXT_MUTED: u32 = 0x707070;
pub const DANGER: u32 = 0xe86060;
pub const WARNING: u32 = 0xe8a860;
pub const STAR: u32 = 0xf0c850;
pub const SELECTION_BAR_WIDTH: f32 = 3.0;
pub const ROUND_SM: f32 = 4.0;
pub const ROUND_MD: f32 = 6.0;
pub const ROUND_LG: f32 = 8.0;
pub const ROUND_XL: f32 = 10.0;
pub const ROUND_PILL: f32 = 999.0;
pub const THUMB_ROUND: f32 = 7.0;
pub const FONT_SIZE_TINY: f32 = 10.5;
pub const FONT_SIZE_SMALL: f32 = 12.0;
pub const FONT_SIZE_BODY: f32 = 13.5;
pub const FONT_SIZE_BUTTON: f32 = 13.0;
pub const FONT_SIZE_HEADING: f32 = 17.0;
pub const FONT_SIZE_TITLE: f32 = 19.0;
pub const UI_FONT_FAMILY: &str = "Segoe UI";
pub const SEMIBOLD_FONT_FAMILY: &str = "Segoe UI Semibold";
pub const ICON_FONT_FAMILY: &str = "Segoe MDL2 Assets";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPreset {
    pub name: &'static str,
    pub color: u32,
}

pub const ACCENT_PRESETS: &[AccentPreset] = &[
    AccentPreset {
        name: "Slate Blue",
        color: 0x5a7fa0,
    },
    AccentPreset {
        name: "Dusty Teal",
        color: 0x5a8a88,
    },
    AccentPreset {
        name: "Muted Copper",
        color: 0xa0654a,
    },
    AccentPreset {
        name: "Dusty Rose",
        color: 0xa07478,
    },
    AccentPreset {
        name: "Sage Green",
        color: 0x6a9a78,
    },
    AccentPreset {
        name: "Dusty Mauve",
        color: 0x8a7098,
    },
    AccentPreset {
        name: "Warm Taupe",
        color: 0x7a6b5e,
    },
];

pub const SYSTEM_ACCENT_NAME: &str = "System";
pub const DEFAULT_ACCENT_NAME: &str = SYSTEM_ACCENT_NAME;
pub const FALLBACK_ACCENT_NAME: &str = "Slate Blue";
static ACCENT: AtomicU32 = AtomicU32::new(0x5a7fa0);
static BACKDROP_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn preset_by_name(name: &str) -> &'static AccentPreset {
    ACCENT_PRESETS
        .iter()
        .find(|preset| preset.name == name)
        .unwrap_or_else(|| {
            ACCENT_PRESETS
                .iter()
                .find(|preset| preset.name == FALLBACK_ACCENT_NAME)
                .unwrap()
        })
}

pub fn normalize_accent(color: u32) -> u32 {
    let [r, g, b] = channels(color);
    let luma = 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
    if luma >= 112.0 {
        return color;
    }
    if luma < 1.0 {
        return preset_by_name(FALLBACK_ACCENT_NAME).color;
    }
    let scale = 112.0 / luma;
    pack(
        (f32::from(r) * scale).min(255.0) as u8,
        (f32::from(g) * scale).min(255.0) as u8,
        (f32::from(b) * scale).min(255.0) as u8,
    )
}

pub fn resolve_accent(name: &str) -> u32 {
    if name == SYSTEM_ACCENT_NAME {
        crate::winfx::system_accent().unwrap_or_else(|| preset_by_name(FALLBACK_ACCENT_NAME).color)
    } else {
        preset_by_name(name).color
    }
}

pub fn accent() -> u32 {
    ACCENT.load(Ordering::Relaxed)
}
pub fn set_accent(color: u32) {
    ACCENT.store(color & 0x00ff_ffff, Ordering::Relaxed);
}
pub fn set_accent_name(name: &str) {
    set_accent(resolve_accent(name));
}
pub fn backdrop_active() -> bool {
    BACKDROP_ACTIVE.load(Ordering::Relaxed)
}
pub fn set_backdrop_active(active: bool) {
    BACKDROP_ACTIVE.store(active, Ordering::Relaxed);
}

/// Source card opacity (232/255 with Mica/Acrylic, opaque otherwise).
pub fn panel_alpha() -> u8 {
    if backdrop_active() { 0xe8 } else { 0xff }
}

pub fn accent_hover() -> u32 {
    let [r, g, b] = channels(accent());
    pack(
        r.saturating_add(24),
        g.saturating_add(24),
        b.saturating_add(24),
    )
}

pub fn accent_alpha(color: u32, alpha: f32) -> u32 {
    let [r, g, b] = channels(color);
    let factor = alpha.clamp(0.0, 1.0);
    pack(
        (f32::from(r) * factor) as u8,
        (f32::from(g) * factor) as u8,
        (f32::from(b) * factor) as u8,
    )
}

pub fn channels(color: u32) -> [u8; 3] {
    [(color >> 16) as u8, (color >> 8) as u8, color as u8]
}

const fn pack(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Stable Windows 10/11 Segoe MDL2 glyph codepoints used by the source UI.
pub mod icons {
    pub const SEARCH: char = '\u{e721}';
    pub const SETTINGS: char = '\u{e713}';
    pub const FILTER: char = '\u{e71c}';
    pub const SORT: char = '\u{e8cb}';
    pub const CALENDAR: char = '\u{e787}';
    pub const FOLDER: char = '\u{e8b7}';
    pub const FOLDER_OPEN: char = '\u{e838}';
    pub const TAG: char = '\u{e8ec}';
    pub const CHEVRON_DOWN: char = '\u{e70d}';
    pub const CHEVRON_RIGHT: char = '\u{e76c}';
    pub const PLAY: char = '\u{e768}';
    pub const CHECK: char = '\u{e73e}';
    pub const STAR: char = '\u{e735}';
    pub const CLOSE: char = '\u{e711}';
    pub const ADD: char = '\u{e710}';
    pub const PICTURE: char = '\u{e8b9}';
    pub const VIDEO: char = '\u{e714}';
    pub const AUDIO: char = '\u{e767}';
    pub const DELETE: char = '\u{e74d}';
    pub const ZOOM: char = '\u{e8a3}';
    pub const VIEW: char = '\u{e7b3}';
    pub const MORE: char = '\u{e712}';
    pub const LIBRARY: char = '\u{e8a9}';
}
