//! PhoTag's shared GPUI design tokens.
//!
//! The v5 interface design (`docs/reference-ui/final-v5/`) defines a neutral
//! graphite palette with slate-blue accents, thin separators and modest
//! 4–6 px corners. Colors are packed `0xRRGGBB` values for direct use with
//! `gpui_kit::rgb`; overlay fills are packed `0xRRGGBBAA` for `gpui_kit::rgba`.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Window root: title band, sidebar backdrop and status bar.
pub const SURFACE_BASE: u32 = 0x22252a;
/// Content surfaces: gallery body and sidebar panel.
pub const SURFACE_PANEL: u32 = 0x272a2f;
/// Cards, inspector panel and raised wells.
pub const SURFACE_CARD: u32 = 0x2b2f34;
/// Hover fills and elevated controls.
pub const SURFACE_ELEVATED: u32 = 0x32363c;
/// Menus, popovers and dialogs.
pub const SURFACE_MODAL: u32 = 0x2d3238;
/// Inset wells: search input, calendar, thumb placeholders.
pub const SURFACE_FAINT: u32 = 0x1e2126;
/// Audio/video stage background, darker than `SURFACE_FAINT`.
pub const SURFACE_AUDIO: u32 = 0x1a1d21;
/// Lowest contrast surface.
pub const SURFACE_EXTREME: u32 = 0x151719;
pub const TEXT_PRIMARY: u32 = 0xe9ebee;
pub const TEXT_SECONDARY: u32 = 0xa9aeb5;
pub const TEXT_MUTED: u32 = 0x7b8189;
pub const DANGER: u32 = 0xe5484d;
pub const DANGER_HOVER: u32 = 0xf06065;
pub const WARNING: u32 = 0xd8a250;
pub const SUCCESS: u32 = 0x3fb56a;
pub const STAR: u32 = 0xf2c14e;

/// Resting fill for chips, rows and ghost controls (`rgba`).
pub const FILL_SUBTLE: u32 = 0xffffff0a;
/// Hover fill (`rgba`).
pub const FILL_HOVER: u32 = 0xffffff12;
/// Pressed/active fill (`rgba`).
pub const FILL_ACTIVE: u32 = 0xffffff1c;
/// Accent-tinted selection fill used for tree rows and segments (`rgba`).
pub const SELECTION_FILL_ALPHA: u32 = 0x3d;
/// Dark hairline between graphite surfaces (`rgb`).
pub const SEPARATOR: u32 = 0x1d2025;
/// Light hairline inside dark wells and menus (`rgba`).
pub const SEPARATOR_LIGHT: u32 = 0xffffff12;
/// Modal scrim over the application (`rgba`).
pub const SCRIM: u32 = 0x0d0f12b8;

// Geometry and typography. Values follow the v5 design direction
// (docs/reference-ui/final-v5/); the source egui constants are superseded.
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 800.0;
pub const WINDOW_MIN_WIDTH: f32 = 960.0;
pub const WINDOW_MIN_HEIGHT: f32 = 600.0;
/// Top band: logo, breadcrumb, thumbnail size, search and Tools.
pub const TITLE_BAND_HEIGHT: f32 = 56.0;
/// Bottom status band.
pub const STATUS_BAR_HEIGHT: f32 = 40.0;
pub const SIDEBAR_WIDTH: f32 = 264.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 200.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 400.0;
pub const SIDEBAR_ROW_HEIGHT: f32 = 30.0;
pub const SIDEBAR_ARROW_SLOT: f32 = 18.0;
/// Right-hand file information column.
pub const INSPECTOR_WIDTH: f32 = 300.0;
pub const SEARCH_FIELD_HEIGHT: f32 = 32.0;
pub const FILTER_CHIP_HEIGHT: f32 = 30.0;
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

// Control fills are light overlays on the v5 graphite surfaces. The legacy
// premultiplied representation is kept so existing call sites compile; the
// packed values resolve to the same `FILL_*` overlays as the rgba tokens.
pub const CONTROL_HOVER: PremultipliedColor = PremultipliedColor {
    red: 20,
    green: 20,
    blue: 20,
    alpha: 20,
};
pub const CONTROL_ACTIVE: PremultipliedColor = PremultipliedColor {
    red: 30,
    green: 30,
    blue: 30,
    alpha: 30,
};
pub const CONTROL_GHOST: PremultipliedColor = PremultipliedColor {
    red: 10,
    green: 10,
    blue: 10,
    alpha: 10,
};
pub const CONTROL_GHOST_HOVER: PremultipliedColor = PremultipliedColor {
    red: 18,
    green: 18,
    blue: 18,
    alpha: 18,
};
pub const CONTROL_GHOST_PRESSED: PremultipliedColor = PremultipliedColor {
    red: 28,
    green: 28,
    blue: 28,
    alpha: 28,
};
pub const CONTROL_BOX: u32 = 0x3a4047;
pub const CONTROL_BOX_HOVER: u32 = 0x464d55;
pub const CONTROL_BOX_PRESSED: u32 = 0x545c66;
pub const HAIRLINE: PremultipliedColor = PremultipliedColor {
    red: 29,
    green: 32,
    blue: 37,
    alpha: 255,
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
pub const THUMB_ROUND: f32 = 6.0;
pub const FONT_SIZE_TINY: f32 = 11.0;
pub const FONT_SIZE_SMALL: f32 = 12.0;
pub const FONT_SIZE_BODY: f32 = 13.0;
pub const FONT_SIZE_BUTTON: f32 = 13.0;
pub const FONT_SIZE_HEADING: f32 = 15.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;
pub const UI_FONT_FAMILY: &str = "Segoe UI";
pub const SEMIBOLD_FONT_FAMILY: &str = "Segoe UI Semibold";
pub const ICON_FONT_FAMILY: &str = "Segoe MDL2 Assets";

/// Apply the v5 theme to GPUI Kit's native controls as well as our paints.
/// Keeps buttons ghost-on-graphite, inputs as faint wells and slate-blue
/// accents for primary actions, selection and focus rings.
pub fn apply_components(cx: &mut gpui_kit::App) {
    use gpui_kit::component::{Theme, ThemeMode};
    use gpui_kit::{px, rgb, rgba};
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.font_family = UI_FONT_FAMILY.into();
        theme.font_size = px(FONT_SIZE_BODY);
        theme.radius = px(ROUND_MD);
        theme.radius_lg = px(ROUND_LG);
        theme.colors.background = rgb(SURFACE_BASE).into();
        theme.colors.foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.button = rgba(FILL_SUBTLE).into();
        theme.colors.button_hover = rgba(FILL_HOVER).into();
        theme.colors.button_active = rgba(FILL_ACTIVE).into();
        theme.colors.button_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.button_secondary = rgba(FILL_SUBTLE).into();
        theme.colors.button_secondary_hover = rgba(FILL_HOVER).into();
        theme.colors.button_secondary_active = rgba(FILL_ACTIVE).into();
        theme.colors.button_secondary_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.border = rgba(SEPARATOR_LIGHT).into();
        theme.colors.input = rgb(SURFACE_FAINT).into();
        theme.colors.secondary = rgb(SURFACE_ELEVATED).into();
        theme.colors.secondary_hover = rgba(FILL_HOVER).into();
        theme.colors.secondary_active = rgba(FILL_ACTIVE).into();
        theme.colors.secondary_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.muted = rgb(SURFACE_PANEL).into();
        theme.colors.muted_foreground = rgb(TEXT_MUTED).into();
        theme.colors.popover = rgb(SURFACE_MODAL).into();
        theme.colors.popover_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.list = rgb(SURFACE_MODAL).into();
        theme.colors.list_hover = rgba(FILL_HOVER).into();
        theme.colors.list_active = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.list_active_border = rgb(accent()).into();
        theme.colors.accent = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.accent_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.selection = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.primary = rgb(accent()).into();
        theme.colors.primary_hover = rgb(accent_hover()).into();
        theme.colors.primary_active = rgb(accent_hover()).into();
        theme.colors.primary_foreground = rgb(0xffffff).into();
        theme.colors.button_primary = rgb(accent()).into();
        theme.colors.button_primary_hover = rgb(accent_hover()).into();
        theme.colors.button_primary_active = rgb(accent_hover()).into();
        theme.colors.button_primary_foreground = rgb(0xffffff).into();
        theme.colors.danger = rgb(DANGER).into();
        theme.colors.danger_hover = rgb(DANGER_HOVER).into();
        theme.colors.danger_active = rgb(DANGER_HOVER).into();
        theme.colors.danger_foreground = rgb(0xffffff).into();
        theme.colors.button_danger = rgb(DANGER).into();
        theme.colors.button_danger_hover = rgb(DANGER_HOVER).into();
        theme.colors.button_danger_active = rgb(DANGER_HOVER).into();
        theme.colors.button_danger_foreground = rgb(0xffffff).into();
        theme.colors.success = rgb(SUCCESS).into();
        theme.colors.success_foreground = rgb(0xffffff).into();
        theme.colors.warning = rgb(WARNING).into();
        theme.colors.warning_foreground = rgb(0xffffff).into();
        theme.colors.caret = rgb(TEXT_PRIMARY).into();
        theme.colors.ring = rgba((accent() << 8) | 0x80).into();
        theme.colors.progress_bar = rgb(accent()).into();
        theme.colors.slider_bar = rgba(FILL_ACTIVE).into();
        theme.colors.slider_thumb = rgb(accent()).into();
        theme.colors.scrollbar = rgba(0).into();
        theme.colors.scrollbar_thumb = rgba(0xffffff2e).into();
        theme.colors.scrollbar_thumb_hover = rgba(0xffffff45).into();
        theme.colors.skeleton = rgb(SURFACE_FAINT).into();
        theme.colors.title_bar = rgb(SURFACE_BASE).into();
        theme.colors.title_bar_border = rgba(SEPARATOR_LIGHT).into();
        theme.colors.status_bar = rgb(SURFACE_BASE).into();
        theme.colors.status_bar_border = rgba(0).into();
        theme.colors.overlay = rgba(SCRIM).into();
        theme.colors.window_border = rgb(SEPARATOR).into();
        theme.colors.group_box = rgb(SURFACE_PANEL).into();
        theme.colors.group_box_foreground = rgb(TEXT_SECONDARY).into();
        theme.colors.description_list_label_foreground = rgb(TEXT_MUTED).into();
        theme.colors.link = rgb(accent()).into();
        theme.colors.link_hover = rgb(accent_hover()).into();
        theme.colors.link_active = rgb(accent_hover()).into();
        theme.colors.sidebar = rgb(SURFACE_PANEL).into();
        theme.colors.sidebar_accent = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.sidebar_accent_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.sidebar_border = rgba(SEPARATOR_LIGHT).into();
        theme.colors.sidebar_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.sidebar_primary = rgb(accent()).into();
        theme.colors.sidebar_primary_foreground = rgb(0xffffff).into();
        theme.colors.tab = rgba(SEPARATOR_LIGHT).into();
        theme.colors.tab_active = rgba(FILL_ACTIVE).into();
        theme.colors.tab_active_foreground = rgb(TEXT_PRIMARY).into();
        theme.colors.tab_bar = rgb(SURFACE_BASE).into();
        theme.colors.tab_foreground = rgb(TEXT_SECONDARY).into();
        theme.colors.switch = rgba(FILL_ACTIVE).into();
        theme.colors.switch_thumb = rgb(TEXT_PRIMARY).into();
        theme.colors.drag_border = rgb(accent()).into();
        theme.colors.drop_target = rgba((accent() << 8) | 0x40).into();
        theme.colors.chart_1 = rgb(accent()).into();
        theme.colors.chart_2 = rgb(0x6a9a78).into();
        theme.colors.chart_3 = rgb(0xa0654a).into();
        theme.colors.chart_4 = rgb(0x8a7098).into();
        theme.colors.chart_5 = rgb(0x5a8a88).into();
        theme.colors.chart_grid = rgba(SEPARATOR_LIGHT).into();
    });
}

/// Keep native controls synchronized with the runtime source accent token.
pub fn apply_accent_components(cx: &mut gpui_kit::App) {
    use gpui_kit::{component::Theme, rgb, rgba};
    Theme::update(cx, |theme| {
        theme.colors.primary = rgb(accent()).into();
        theme.colors.primary_hover = rgb(accent_hover()).into();
        theme.colors.button_primary = rgb(accent()).into();
        theme.colors.button_primary_hover = rgb(accent_hover()).into();
        theme.colors.accent = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.selection = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.list_active = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.list_active_border = rgb(accent()).into();
        theme.colors.ring = rgba((accent() << 8) | 0x80).into();
        theme.colors.progress_bar = rgb(accent()).into();
        theme.colors.slider_thumb = rgb(accent()).into();
        theme.colors.link = rgb(accent()).into();
        theme.colors.link_hover = rgb(accent_hover()).into();
        theme.colors.sidebar_accent = rgba((accent() << 8) | SELECTION_FILL_ALPHA).into();
        theme.colors.chart_1 = rgb(accent()).into();
    });
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

/// Parse "#RRGGBB" → `[r, g, b]`; `None` on malformed input.
/// `vendor/photag-source/src/ui/sidebar.rs:8-15`.
pub fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}

/// `vendor/photag-source/src/ui/sidebar.rs:44-50`: per-tag icon tint,
/// falling back to the neutral gray used when a tag has no color.
pub fn tag_tint(color: Option<&str>) -> u32 {
    color
        .and_then(parse_hex_color)
        .map_or(0x78787a, |[r, g, b]| pack(r, g, b))
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
    pub const WARNING: char = '\u{e7ba}';
    pub const EXTENSION: char = '\u{e8a5}';
}
