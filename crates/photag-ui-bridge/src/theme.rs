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
/// Source audio-preview stage background (`Color32::from_gray(24)`).
pub const SURFACE_AUDIO: u32 = 0x181818;
/// Lowest contrast surface used for egui's extreme/faint backgrounds.
pub const SURFACE_EXTREME: u32 = 0x131313;
pub const TEXT_PRIMARY: u32 = 0xebebeb;
pub const TEXT_SECONDARY: u32 = 0xa0a0a0;
pub const TEXT_MUTED: u32 = 0x707070;
pub const DANGER: u32 = 0xe86060;
pub const WARNING: u32 = 0xe8a860;
pub const STAR: u32 = 0xf0c850;

// Geometry and typography copied from vendor/photag-source/src/main.rs,
// ui/filterbar.rs, ui/sidebar.rs, app.rs, and theme.rs. Feature views should
// use these source values instead of deriving sizes from screenshots.
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 800.0;
pub const WINDOW_MIN_WIDTH: f32 = 800.0;
pub const WINDOW_MIN_HEIGHT: f32 = 500.0;
pub const SIDEBAR_WIDTH: f32 = 220.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 160.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 500.0;
pub const SIDEBAR_ROW_HEIGHT: f32 = 26.0;
pub const SIDEBAR_ARROW_SLOT: f32 = 16.0;
pub const SEARCH_FIELD_HEIGHT: f32 = 32.0;
pub const FILTER_CHIP_HEIGHT: f32 = 26.0;
pub const THUMB_SIZE_MIN: f32 = 80.0;
pub const THUMB_SIZE_MAX: f32 = 300.0;
pub const THUMB_SIZE_DEFAULT: f32 = 150.0;
pub const THUMB_SLIDER_WIDTH: f32 = 116.0;
pub const SOURCE_FONT_SIZE_BUTTON: f32 = 13.0;
pub const SOURCE_ITEM_SPACING_X: f32 = 8.0;
pub const SOURCE_ITEM_SPACING_Y: f32 = 6.0;
pub const SOURCE_BUTTON_PADDING_X: f32 = 12.0;
pub const SOURCE_BUTTON_PADDING_Y: f32 = 5.0;
pub const SOURCE_MENU_MARGIN_X: f32 = 10.0;
pub const SOURCE_MENU_MARGIN_Y: f32 = 8.0;
pub const SOURCE_WINDOW_MARGIN: f32 = 14.0;
pub const SOURCE_SCROLLBAR_WIDTH: f32 = 7.0;
/// Source colors are premultiplied because they are composited over the
/// active surface. Keep alpha explicit when adapting them to GPUI paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PremultipliedColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl PremultipliedColor {
    /// Pack this source color as the straight-alpha `0xRRGGBBAA` value that
    /// GPUI's `rgba` helper expects. Source egui tokens are already premultiplied.
    pub const fn to_rgba(self) -> u32 {
        if self.alpha == 0 {
            return 0;
        }
        ((unpremultiply(self.red, self.alpha) as u32) << 24)
            | ((unpremultiply(self.green, self.alpha) as u32) << 16)
            | ((unpremultiply(self.blue, self.alpha) as u32) << 8)
            | self.alpha as u32
    }
}

const fn unpremultiply(channel: u8, alpha: u8) -> u8 {
    let value = (channel as u32 * 255 + alpha as u32 / 2) / alpha as u32;
    if value > 255 { 255 } else { value as u8 }
}

pub const CONTROL_HOVER: PremultipliedColor = PremultipliedColor {
    red: 22,
    green: 22,
    blue: 22,
    alpha: 22,
};
pub const CONTROL_ACTIVE: PremultipliedColor = PremultipliedColor {
    red: 12,
    green: 12,
    blue: 12,
    alpha: 12,
};
pub const CONTROL_GHOST: PremultipliedColor = PremultipliedColor {
    red: 16,
    green: 16,
    blue: 16,
    alpha: 16,
};
pub const CONTROL_GHOST_HOVER: PremultipliedColor = PremultipliedColor {
    red: 28,
    green: 28,
    blue: 28,
    alpha: 28,
};
pub const CONTROL_GHOST_PRESSED: PremultipliedColor = PremultipliedColor {
    red: 10,
    green: 10,
    blue: 10,
    alpha: 10,
};
pub const CONTROL_BOX: u32 = 0x4a4a4a;
pub const CONTROL_BOX_HOVER: u32 = 0x5c5c5c;
pub const CONTROL_BOX_PRESSED: u32 = 0x686868;
pub const HAIRLINE: PremultipliedColor = PremultipliedColor {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 64,
};
pub const BORDER: PremultipliedColor = PremultipliedColor {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 130,
};
pub const FAINT_OVERLAY: PremultipliedColor = PremultipliedColor {
    red: 8,
    green: 8,
    blue: 8,
    alpha: 8,
};
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

/// Apply the pinned theme to GPUI Kit's native controls as well as our paints.
/// Source theme.rs:408-534: ghost controls, borderless inputs, Segoe typography.
pub fn apply_components(cx: &mut gpui_kit::App) {
    use gpui_kit::component::{Theme, ThemeMode};
    use gpui_kit::{px, rgb, rgba};
    Theme::change(ThemeMode::Dark, None, cx);
    let theme = Theme::global_mut(cx);
    theme.font_family = UI_FONT_FAMILY.into();
    theme.font_size = px(FONT_SIZE_BODY);
    theme.radius = px(ROUND_SM + 1.);
    theme.radius_lg = px(ROUND_LG);
    theme.colors.background = rgb(SURFACE_BASE).into();
    theme.colors.foreground = rgb(TEXT_PRIMARY).into();
    theme.colors.button = rgba(CONTROL_GHOST.to_rgba()).into();
    theme.colors.button_hover = rgba(CONTROL_GHOST_HOVER.to_rgba()).into();
    theme.colors.button_active = rgba(CONTROL_GHOST_PRESSED.to_rgba()).into();
    theme.colors.button_foreground = rgb(TEXT_PRIMARY).into();
    theme.colors.border = rgba(0).into();
    theme.colors.input = rgba(0).into();
    theme.colors.secondary = rgb(SURFACE_PANEL).into();
    theme.colors.secondary_foreground = rgb(TEXT_PRIMARY).into();
    theme.colors.muted_foreground = rgb(TEXT_MUTED).into();
    theme.colors.popover = rgb(SURFACE_MODAL).into();
    theme.colors.popover_foreground = rgb(TEXT_PRIMARY).into();
    theme.colors.list = rgb(SURFACE_MODAL).into();
    theme.colors.list_hover = rgba(CONTROL_HOVER.to_rgba()).into();
    theme.colors.accent = rgba((accent() << 8) | 64).into();
    theme.colors.accent_foreground = rgb(TEXT_PRIMARY).into();
    theme.colors.selection = rgba((accent() << 8) | 64).into();
    theme.colors.primary = rgb(accent()).into();
    theme.colors.caret = rgb(TEXT_PRIMARY).into();
    theme.colors.ring = rgba(0).into();
}

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
    // Match PhoTag's winfx brightness lift: preserve the source hue by
    // scaling all channels until the brightest channel reaches 150.
    const MIN_BRIGHTEST_CHANNEL: u32 = 150;
    let brightest = u32::from(r.max(g).max(b));
    if brightest >= MIN_BRIGHTEST_CHANNEL {
        return color;
    }
    if brightest == 0 {
        return 0x969696;
    }
    let scale = MIN_BRIGHTEST_CHANNEL;
    pack(
        ((u32::from(r) * scale) / brightest).min(255) as u8,
        ((u32::from(g) * scale) / brightest).min(255) as u8,
        ((u32::from(b) * scale) / brightest).min(255) as u8,
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

/// Source base-panel opacity (222/255 with Mica/Acrylic, opaque otherwise).
pub fn base_rgba(color: u32) -> u32 {
    (color << 8) | if backdrop_active() { 0xde } else { 0xff }
}

/// Source modal/window opacity (244/255 with Mica/Acrylic, opaque otherwise).
pub fn modal_rgba(color: u32) -> u32 {
    (color << 8) | if backdrop_active() { 0xf4 } else { 0xff }
}

/// Pack an RGB surface token with the application backdrop-aware panel alpha.
pub fn panel_rgba(color: u32) -> u32 {
    (color << 8) | u32::from(panel_alpha())
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
