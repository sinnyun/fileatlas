//! GPUI Kit is the only palette and theme resolver. This module provides
//! geometry and role aliases for custom views; aliases read the native Theme
//! projection and never define independent dark/light colors.
use gpui_kit::Styled as _;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
mod color;
mod contrast;
mod roles;
mod runtime;
pub use color::PremultipliedColor;
pub use contrast::*;
pub use roles::*;
pub use runtime::*;
static BACKDROP_ACTIVE: AtomicBool = AtomicBool::new(false);
pub fn backdrop_active() -> bool {
    BACKDROP_ACTIVE.load(Ordering::Relaxed)
}
pub fn set_backdrop_active(active: bool) {
    BACKDROP_ACTIVE.store(active, Ordering::Relaxed);
}
// Legacy persisted accent names are accepted as metadata, never used as a palette.
pub const DEFAULT_ACCENT_NAME: &str = "Workbench Blue";
pub const SYSTEM_ACCENT_NAME: &str = "System";
pub fn normalize_accent(color: u32) -> u32 {
    color
}
pub const SPACE_1: f32 = 4.;
pub const SPACE_2: f32 = 8.;
pub const SPACE_3: f32 = 12.;
pub const SPACE_4: f32 = 16.;
pub const PANEL_GAP: f32 = 10.;
pub const NAV_ROW_HEIGHT: f32 = 40.;
pub const ACTION_HEIGHT: f32 = 38.;
pub const ICON_BUTTON_SIZE: f32 = 30.;

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
    let region = region.border_color(gpui_kit::rgba(region_edge()));
    match edge {
        RegionEdge::All => region.border_1(),
        RegionEdge::Left => region.border_l_1(),
        RegionEdge::Right => region.border_r_1(),
        RegionEdge::Top => region.border_t_1(),
        RegionEdge::Bottom => region.border_b_1(),
    }
}

fn region_shadows(edge: RegionEdge) -> Vec<gpui_kit::BoxShadow> {
    use gpui_kit::{BoxShadow, px, rgba};
    let (offset_x, offset_y) = match edge {
        RegionEdge::All => (0., 6.),
        RegionEdge::Left => (-10., 0.),
        RegionEdge::Right => (10., 0.),
        RegionEdge::Top => (0., -10.),
        RegionEdge::Bottom => (0., 10.),
    };
    vec![
        BoxShadow::new(px(offset_x), px(offset_y), rgba(shadow_overlay()).into())
            .blur_radius(px(18.))
            .spread_radius(px(-2.)),
    ]
}

/// Compatibility RGB adapters preserve native Theme alpha, never inject backdrop colors.
pub fn panel_alpha() -> u8 {
    (native_colors().group_box.a * 255.).round() as u8
}
pub fn base_rgba(color: u32) -> u32 {
    let native = native_colors();
    if color == packed(native.background) {
        packed_rgba(native.background)
    } else {
        (color << 8) | 255
    }
}
pub fn modal_rgba(color: u32) -> u32 {
    let native = native_colors();
    if color == packed(native.popover) {
        packed_rgba(native.popover)
    } else {
        (color << 8) | 255
    }
}
pub fn panel_rgba(color: u32) -> u32 {
    let native = native_colors();
    if color == packed(native.group_box) {
        packed_rgba(native.group_box)
    } else {
        base_rgba(color)
    }
}
pub fn group_tint(index: usize) -> u32 {
    let colors = native_colors();
    let palette = [
        colors.chart_1,
        colors.chart_2,
        colors.chart_3,
        colors.chart_4,
        colors.chart_5,
    ];
    packed(palette[index % palette.len()])
}
pub fn channels(color: u32) -> [u8; 3] {
    [(color >> 16) as u8, (color >> 8) as u8, color as u8]
}

/// Parse "#RRGGBB" → `[r, g, b]`; `None` on malformed input.
/// `vendor/photag-source/src/ui/sidebar.rs:8-15`.
pub fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 || !value.is_ascii() {
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
        .map_or(text_muted(), |[r, g, b]| pack(r, g, b))
}

const fn pack(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}
