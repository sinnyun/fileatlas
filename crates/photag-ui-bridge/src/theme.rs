//! PhoTag's shared GPUI design tokens.
//!
//! The workbench theme follows the clipboard v2 navy palette.
//! See `docs/ui-theme.md` for component, layout and contribution rules. Colors are packed `0xRRGGBB` values for direct use with
//! `gpui_kit::rgb`; overlay fills are packed `0xRRGGBBAA` for `gpui_kit::rgba`.

use gpui_kit::Styled as _;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Window root: title band, sidebar backdrop and status bar.
pub const SURFACE_BASE: u32 = 0x0e1823;
/// Content surfaces: gallery body and sidebar panel.
pub const SURFACE_PANEL: u32 = 0x152230;
/// Cards, inspector panel and raised wells.
pub const SURFACE_CARD: u32 = 0x1b2a3b;
/// Hover fills and elevated controls.
pub const SURFACE_ELEVATED: u32 = 0x24374b;
/// Menus, popovers and dialogs.
pub const SURFACE_MODAL: u32 = 0x152230;
/// Inset wells: search input, calendar, thumb placeholders.
pub const SURFACE_FAINT: u32 = 0x101b27;
/// Audio/video stage background, darker than `SURFACE_FAINT`.
pub const SURFACE_AUDIO: u32 = 0x101b27;
/// Lowest contrast surface.
pub const SURFACE_EXTREME: u32 = 0x0a121b;
/// Leftmost navigation rail: darker than the window base so the rail, the
/// sidebar and the content column read as three separate regions.
pub const SURFACE_RAIL: u32 = 0x0b141e;
/// Sidebar panel between the rail and the content column.
pub const SURFACE_SIDEBAR: u32 = 0x152230;
/// Inset well behind a bounded sidebar region (drives, favorites, …).
pub const SURFACE_REGION: u32 = 0x101b27;
/// Region rule in border mode, as 0xRRGGBBAA.
pub const REGION_EDGE: u32 = 0x304357ff;
/// Raised band behind a browser group title.
pub const SURFACE_GROUP_HEADER: u32 = 0x1b2a3b;
/// Hover fill for a browser group title.
pub const SURFACE_GROUP_HEADER_HOVER: u32 = 0x24374b;
pub const TEXT_PRIMARY: u32 = 0xe7eef7;
pub const TEXT_SECONDARY: u32 = 0x9eafc5;
pub const TEXT_MUTED: u32 = 0x8195ad;
pub const DANGER: u32 = 0xe5484d;
pub const DANGER_HOVER: u32 = 0xf06065;
pub const WARNING: u32 = 0xe4ac55;
pub const SUCCESS: u32 = 0x32cd8b;
pub const STAR: u32 = 0xf6c654;

/// Resting fill for chips, rows and ghost controls (`rgba`).
pub const FILL_SUBTLE: u32 = 0xffffff0a;
/// Hover fill (`rgba`).
pub const FILL_HOVER: u32 = 0xffffff12;
/// Pressed/active fill (`rgba`).
pub const FILL_ACTIVE: u32 = 0xffffff1c;
/// Accent-tinted selection fill used for tree rows and segments (`rgba`).
pub const SELECTION_FILL_ALPHA: u32 = 0x3d;
/// Navy hairline between surfaces (`rgb`).
pub const SEPARATOR: u32 = 0x304357;
/// Light hairline inside dark wells and menus (`rgba`).
pub const SEPARATOR_LIGHT: u32 = 0x304357ff;
/// Modal scrim over the application (`rgba`).
pub const SCRIM: u32 = 0x060d17b8;

/// Semantic colors and overlays for custom-painted controls and media badges.
pub const TEXT_ON_ACCENT: u32 = 0xffffff;
pub const BRANCH_ALTERNATE: u32 = 0xb574ec;
pub const INFO: u32 = 0x2787ff;
pub const CATEGORY: u32 = 0xb379e8;
pub const DATA: u32 = 0x38b9ce;
pub const MEDIA_SCRIM: u32 = 0x00000096;
pub const MEDIA_SCRIM_STRONG: u32 = 0x000000d8;
pub const TOOLTIP_SURFACE: u32 = 0x101b27ed;
pub const SHADOW_OVERLAY: u32 = 0x00000060;
pub const TRACK_SHADOW: u32 = 0x0000006e;
pub const TRACK_HIGHLIGHT: u32 = 0xffffff16;
pub const TRACK_ACTIVE: u32 = 0xffffff28;
pub const FILL_TRANSPARENT_WHITE: u32 = 0xffffff00;
pub const BORDER_STRONG: u32 = 0x43566dff;
pub const SPACE_1: f32 = 4.;
pub const SPACE_2: f32 = 8.;
pub const SPACE_3: f32 = 12.;
pub const SPACE_4: f32 = 16.;
pub const PANEL_GAP: f32 = 10.;
pub const NAV_ROW_HEIGHT: f32 = 40.;
pub const ACTION_HEIGHT: f32 = 38.;
pub const ICON_BUTTON_SIZE: f32 = 30.;

/// Opaque selection paint is shared across cards/navigation and preserves user accents.
pub fn selection_color() -> u32 {
    if accent() == 0x2787ff {
        return 0x163b63;
    }
    let [r, g, b] = channels(accent());
    let [br, bg, bb] = channels(SURFACE_PANEL);
    let mix = |base: u8, color: u8| ((u32::from(base) * 3 + u32::from(color)) / 4) as u8;
    pack(mix(br, r), mix(bg, g), mix(bb, b))
}

// Shared workbench geometry; feature-specific layouts keep their own constraints.
/// Lower bound for the startup window; the actual size grows with the display.
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 800.0;
pub const WINDOW_MIN_WIDTH: f32 = 960.0;
pub const WINDOW_MIN_HEIGHT: f32 = 600.0;
/// Share of the primary display work area requested at startup.
pub const WINDOW_WORK_AREA_FRACTION: f32 = 0.85;
/// Top band: logo, breadcrumb, thumbnail size, search and Tools.
pub const TITLE_BAND_HEIGHT: f32 = 56.0;
/// Bottom status band.
pub const STATUS_BAR_HEIGHT: f32 = 40.0;
pub const SIDEBAR_WIDTH: f32 = 264.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 200.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 400.0;
pub const SIDEBAR_ROW_HEIGHT: f32 = 30.0;
pub const SIDEBAR_ARROW_SLOT: f32 = 18.0;
/// Height of a browser group title band.
pub const GROUP_HEADER_HEIGHT: f32 = 40.0;
/// Vertical gap between a group title band and that group's content cards.
pub const GROUP_HEADER_GAP: f32 = 10.0;
/// Vertical gap between two consecutive browser groups.
pub const GROUP_TOP_GAP: f32 = 18.0;
/// Height of a sidebar region header row.
pub const REGION_HEADER_HEIGHT: f32 = 26.0;
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
mod color;
pub use color::PremultipliedColor;

// Control fills are light overlays on workbench surfaces. The legacy
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
pub const CONTROL_BOX: u32 = 0x304357;
pub const CONTROL_BOX_HOVER: u32 = 0x3d536c;
pub const CONTROL_BOX_PRESSED: u32 = 0x4b6380;
pub const HAIRLINE: PremultipliedColor = PremultipliedColor {
    red: 48,
    green: 67,
    blue: 87,
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
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_BUTTON: f32 = 14.0;
pub const FONT_SIZE_HEADING: f32 = 15.0;
pub const FONT_SIZE_TITLE: f32 = 18.0;
pub const UI_FONT_FAMILY: &str = "Segoe UI";
/// Settings concepts use a sans-serif CJK face rather than Windows' serif fallback.
pub const SETTINGS_FONT_FAMILY: &str = "Microsoft YaHei UI";
/// Shared settings geometry, calibrated against the complete minimal concepts.
pub const SETTINGS_NAV_WIDTH: f32 = 224.;
pub const SETTINGS_CONTENT_PADDING: f32 = 40.;
pub const SETTINGS_TITLE_SIZE: f32 = 24.;
pub const SETTINGS_SECTION_SIZE: f32 = 16.;
pub const SETTINGS_CONTROL_HEIGHT: f32 = 36.;
pub const SEMIBOLD_FONT_FAMILY: &str = "Segoe UI Semibold";
/// Shared icon dimensions in logical pixels. Keep UI icon sizing here so
/// controls remain visually consistent across feature crates.
pub const ICON_SIZE_SMALL: f32 = 12.0;
pub const ICON_SIZE_DEFAULT: f32 = 16.0;
pub const ICON_SIZE_LARGE: f32 = 20.0;
pub const ICON_SIZE_EMPTY_STATE: f32 = 40.0;

/// Build a standard-sized icon from the bundled GPUI Kit/Lucide catalog.
pub fn icon(name: gpui_kit::assets::IconName) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::new(name).size(gpui_kit::px(ICON_SIZE_DEFAULT))
}

/// Build a compact icon for dense rows and popovers.
pub fn small_icon(name: gpui_kit::assets::IconName) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::new(name).size(gpui_kit::px(ICON_SIZE_SMALL))
}

/// Build a large icon for prominent empty-state or modal feedback.
pub fn large_icon(name: gpui_kit::assets::IconName) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::new(name).size(gpui_kit::px(ICON_SIZE_LARGE))
}

pub fn empty_state_icon(name: gpui_kit::assets::IconName) -> gpui_kit::component::Icon {
    gpui_kit::component::Icon::new(name).size(gpui_kit::px(ICON_SIZE_EMPTY_STATE))
}

/// Resolve persisted tag icon identifiers to bundled icons. Old versions
/// stored Segoe glyphs and emoji, so unknown legacy values fall back safely.
pub fn tag_icon(name: Option<&str>) -> gpui_kit::assets::IconName {
    use gpui_kit::assets::IconName;
    match name.unwrap_or_default() {
        "Image" | "\u{e8b9}" => IconName::Image,
        "Folder" | "\u{e8b7}" => IconName::Folder,
        "Video" | "\u{e714}" => IconName::Video,
        "Music" | "\u{e767}" => IconName::Music,
        "Star" | "\u{e735}" => IconName::Star,
        "Search" | "\u{e721}" => IconName::Search,
        "Library" | "\u{e8a9}" => IconName::Library,
        "Settings" | "\u{e713}" => IconName::Settings,
        "Puzzle" | "\u{e8a5}" => IconName::Puzzle,
        "ZoomIn" | "\u{e8a3}" => IconName::ZoomIn,
        "Eye" | "\u{e7b3}" => IconName::Eye,
        "Calendar" | "\u{e787}" => IconName::Calendar,
        "ListFilter" | "\u{e71c}" => IconName::ListFilter,
        "ArrowUpDown" | "\u{e8cb}" => IconName::ArrowUpDown,
        _ => IconName::Tag,
    }
}

mod components;
pub use components::{apply_accent_components, apply_components};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPreset {
    pub name: &'static str,
    pub color: u32,
}

pub const ACCENT_PRESETS: &[AccentPreset] = &[
    AccentPreset {
        name: "Workbench Blue",
        color: 0x2787ff,
    },
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
pub const DEFAULT_ACCENT_NAME: &str = "Workbench Blue";
pub const FALLBACK_ACCENT_NAME: &str = DEFAULT_ACCENT_NAME;
static ACCENT: AtomicU32 = AtomicU32::new(0x2787ff);
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

/// How the application separates its regions from each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionDepth {
    /// Hairline rule on the shared edge.
    Border,
    /// Drop shadow cast toward the neighbouring region.
    Shadow,
}

pub const REGION_DEPTH_BORDER: &str = "border";
pub const REGION_DEPTH_SHADOW: &str = "shadow";
static REGION_DEPTH: AtomicU32 = AtomicU32::new(0);

pub fn region_depth() -> RegionDepth {
    if REGION_DEPTH.load(Ordering::Relaxed) == 1 {
        RegionDepth::Shadow
    } else {
        RegionDepth::Border
    }
}

/// Name form stored in the settings document.
pub fn region_depth_name() -> &'static str {
    match region_depth() {
        RegionDepth::Border => REGION_DEPTH_BORDER,
        RegionDepth::Shadow => REGION_DEPTH_SHADOW,
    }
}

/// Apply a persisted settings value; unknown values keep the hairline default.
pub fn set_region_depth_name(name: &str) {
    REGION_DEPTH.store(u32::from(name == REGION_DEPTH_SHADOW), Ordering::Relaxed);
}

/// Sides of a region that carry the separation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionEdge {
    All,
    Left,
    Right,
    Top,
    Bottom,
}

/// Apply the configured region separation to a panel. Shadow mode replaces
/// the rule with a soft drop shadow, so callers must not add their own border.
pub fn separate_region(region: gpui_kit::Div, edge: RegionEdge) -> gpui_kit::Div {
    if region_depth() == RegionDepth::Shadow {
        return region.shadow(region_shadows(edge));
    }
    let region = region.border_color(gpui_kit::rgba(REGION_EDGE));
    match edge {
        RegionEdge::All => region.border_1(),
        RegionEdge::Left => region.border_l_1(),
        RegionEdge::Right => region.border_r_1(),
        RegionEdge::Top => region.border_t_1(),
        RegionEdge::Bottom => region.border_b_1(),
    }
}

fn region_shadows(edge: RegionEdge) -> Vec<gpui_kit::BoxShadow> {
    use gpui_kit::{BoxShadow, hsla, px};
    let (offset_x, offset_y) = match edge {
        RegionEdge::All => (0., 6.),
        RegionEdge::Left => (-10., 0.),
        RegionEdge::Right => (10., 0.),
        RegionEdge::Top => (0., -10.),
        RegionEdge::Bottom => (0., 10.),
    };
    vec![
        BoxShadow::new(px(offset_x), px(offset_y), hsla(0., 0., 0., 0.72))
            .blur_radius(px(18.))
            .spread_radius(px(-2.)),
    ]
}

/// Header tints used to tell the custom sidebar groups apart.
pub const GROUP_TINTS: &[u32] = &[0x5a7fa0, 0x6a9a78, 0xa0654a, 0x8a7098, 0x5a8a88, 0xa07478];

/// Deterministic tint for the group at `index`.
pub fn group_tint(index: usize) -> u32 {
    GROUP_TINTS[index % GROUP_TINTS.len()]
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
