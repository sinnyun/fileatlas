//! GPUI Kit is the only palette and theme resolver. This module provides
//! geometry and role aliases for custom views; aliases read the native Theme
//! projection and never define independent dark/light colors.
use gpui_kit::Styled as _;
use gpui_kit::{BoxShadow, px, rgba};
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
static UI_DENSITY: AtomicU32 = AtomicU32::new(100);
static GEOMETRY_AUDIT_ENABLED: AtomicBool = AtomicBool::new(true);
static TYPOGRAPHY_AUDIT_ENABLED: AtomicBool = AtomicBool::new(true);
pub fn backdrop_active() -> bool {
    BACKDROP_ACTIVE.load(Ordering::Relaxed)
}
pub fn set_backdrop_active(active: bool) {
    BACKDROP_ACTIVE.store(active, Ordering::Relaxed);
}
/// Whether shared component review outlines and geometry probes are active.
pub fn geometry_audit_enabled() -> bool {
    GEOMETRY_AUDIT_ENABLED.load(Ordering::Relaxed)
}
/// Update the shared review mode. Settings owns persistence; this module owns
/// the process-wide rendering switch used by every window and plugin.
pub fn set_geometry_audit_enabled(enabled: bool) {
    GEOMETRY_AUDIT_ENABLED.store(enabled, Ordering::Relaxed);
}
/// Whether temporary colors identifying the three typography tiers are active.
pub fn typography_audit_enabled() -> bool {
    TYPOGRAPHY_AUDIT_ENABLED.load(Ordering::Relaxed)
}
/// Toggle typography review separately from geometry outlines.
pub fn set_typography_audit_enabled(enabled: bool) {
    TYPOGRAPHY_AUDIT_ENABLED.store(enabled, Ordering::Relaxed);
    runtime::set_typography_audit_enabled(enabled);
}
// Legacy persisted accent names are accepted as metadata, never used as a palette.
pub const DEFAULT_ACCENT_NAME: &str = "Workbench Blue";
pub const SYSTEM_ACCENT_NAME: &str = "System";
pub fn normalize_accent(color: u32) -> u32 {
    color
}
/// The only product spacing tiers. Settings scales the shared values together.
pub fn set_ui_density(density: &str) {
    let scale = match density {
        "compact" => 90,
        "comfortable" => 110,
        _ => 100,
    };
    UI_DENSITY.store(scale, Ordering::Relaxed);
}
fn density_scale() -> f32 {
    UI_DENSITY.load(Ordering::Relaxed) as f32 / 100.
}
pub fn space_compact() -> f32 {
    4. * density_scale()
}
pub fn space_standard() -> f32 {
    8. * density_scale()
}
pub fn space_expanded() -> f32 {
    16. * density_scale()
}
pub fn ui_height(value: f32) -> f32 {
    value * density_scale()
}
pub fn button_height() -> f32 {
    BUTTON_HEIGHT * density_scale()
}
pub fn nav_row_height() -> f32 {
    NAV_ROW_HEIGHT * density_scale()
}
pub const NAV_ROW_HEIGHT: f32 = 40.;
pub const ACTION_HEIGHT: f32 = 38.;
pub const ICON_BUTTON_SIZE: f32 = 30.;
/// Vertical launcher rail entries share one footprint across feature plugins.
pub const RAIL_SHORTCUT_WIDTH: f32 = 42.;
pub const RAIL_SHORTCUT_HEIGHT: f32 = 54.;
pub const RAIL_FOOTER_WIDTH: f32 = 48.;
pub const RAIL_FOOTER_HEIGHT: f32 = 96.;
/// Shared semantic control heights used by native controls and plugin layouts.
pub const CONTROL_HEIGHT_COMPACT: f32 = 26.;
pub const CONTROL_HEIGHT_SMALL: f32 = 28.;
pub const CONTROL_HEIGHT_DENSE: f32 = 30.;
pub const CONTROL_HEIGHT_NAV: f32 = 40.;
pub const CONTROL_HEIGHT_STANDARD: f32 = 32.;
/// Standard multiline text editing surface height.
pub const TEXTAREA_HEIGHT_STANDARD: f32 = 120.;
pub const CONTROL_HEIGHT_LARGE: f32 = 38.;
pub const CONTROL_HEIGHT_TOOLBAR: f32 = 34.;
pub const PROGRAM_CARD_HEIGHT: f32 = 180.;
pub const PROGRAM_CARD_NAME_HEIGHT: f32 = 80.;
pub const CONTROL_HEIGHT_FOOTER: f32 = 48.;
pub const KEYCAP_WIDTH_STANDARD: f32 = 114.;
pub const KEYCAP_WIDTH_COMPACT: f32 = 32.;
pub const KEYCAP_WIDTH_TAG: f32 = 54.;
pub const KEYCAP_HEIGHT_TAG: f32 = 24.;
pub const CANVAS_SELECTION_HANDLE_SIZE: f32 = 7.;
pub const TIMELINE_CORNER_RADIUS: f32 = 25.;
pub const TIMELINE_RAIL_WIDTH: f32 = 2.;
pub const CONTROL_WIDTH_NARROW: f32 = 96.;
pub const CONTROL_WIDTH_STANDARD: f32 = 200.;
pub const CONTROL_WIDTH_WIDE: f32 = 240.;
pub const CONTROL_WIDTH_EXTRA_WIDE: f32 = 320.;

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
/// Height of a sidebar region header row.
pub const REGION_HEADER_HEIGHT: f32 = 26.0;
/// Right-hand file information column.
pub const INSPECTOR_WIDTH: f32 = 300.0;
/// All shared buttons have one height; labels/icons determine width after this minimum.
pub const BUTTON_HEIGHT: f32 = 32.0;
pub const BUTTON_MIN_WIDTH: f32 = 32.0;
pub const DIVIDER_THICKNESS: f32 = 1.0;
pub const FILTER_CHIP_HEIGHT: f32 = 30.0;
pub const THUMB_SIZE_MIN: f32 = 80.0;
pub const THUMB_SIZE_MAX: f32 = 300.0;
pub const THUMB_SIZE_DEFAULT: f32 = 150.0;
pub const THUMB_SLIDER_WIDTH: f32 = 116.0;
pub const SLIDER_THUMB_SIZE: f32 = 16.0;
pub const SLIDER_THUMB_TOP: f32 = 3.0;
pub const SLIDER_THUMB_RING: f32 = 1.5;
pub const SLIDER_THUMB_DOT: f32 = 4.8;
pub const SLIDER_RAIL_TOP: f32 = 9.5;
pub const SLIDER_RAIL_HEIGHT: f32 = 3.0;
pub const MEDIA_BADGE_HEIGHT: f32 = 18.0;
pub const SOURCE_ITEM_SPACING_X: f32 = 8.0;
pub const SOURCE_ITEM_SPACING_Y: f32 = 6.0;
pub const SOURCE_BUTTON_PADDING_X: f32 = 12.0;
pub const SOURCE_BUTTON_PADDING_Y: f32 = 5.0;
pub const SOURCE_MENU_MARGIN_X: f32 = 10.0;
pub const SOURCE_MENU_MARGIN_Y: f32 = 8.0;
pub const SOURCE_WINDOW_MARGIN: f32 = 14.0;
pub const SOURCE_SCROLLBAR_WIDTH: f32 = 7.0;
pub const SELECTION_BAR_WIDTH: f32 = 3.0;
pub const ROUND_MICRO: f32 = 2.0;
pub const ROUND_SM: f32 = 4.0;
pub const ROUND_MD: f32 = 6.0;
pub const ROUND_LG: f32 = 8.0;
pub const ROUND_XL: f32 = 10.0;
pub const ROUND_PILL: f32 = 999.0;
pub const THUMB_ROUND: f32 = 6.0;
pub const FONT_SIZE_ASSISTANT: f32 = 12.0;
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_TITLE: f32 = 15.0;
pub const LINE_HEIGHT_TITLE: f32 = 20.0;
pub const LINE_HEIGHT_BODY: f32 = 18.0;
pub const LINE_HEIGHT_ASSISTANT: f32 = 16.0;
/// Temporary geometry-review colors. Remove only after the user accepts the audit.
pub const AUDIT_FRAME_OUTER: u32 = 0xE60000;
pub const AUDIT_FRAME_SECTION: u32 = 0xFF8C00;
pub const AUDIT_FRAME_CARD: u32 = 0xD6B600;
pub const AUDIT_FRAME_NAV: u32 = 0x00A040;
pub const AUDIT_FRAME_BUTTON: u32 = 0x0066CC;
pub const AUDIT_FRAME_INPUT: u32 = 0xC000C0;
/// Cyan outlines identify the occupied bounds of shared typography wrappers.
pub const AUDIT_FRAME_TEXT: u32 = 0x00BFD8;
pub const AUDIT_TEXT_TITLE: u32 = 0xE60000;
pub const AUDIT_TEXT_BODY: u32 = 0x00A040;
pub const AUDIT_TEXT_ASSISTANT: u32 = 0x0066CC;
pub const UI_FONT_FAMILY: &str = "Segoe UI";
/// Shared settings geometry, calibrated against the complete minimal concepts.
pub const SETTINGS_NAV_WIDTH: f32 = 224.;
pub const SETTINGS_CONTENT_PADDING: f32 = 40.;
pub const SETTINGS_CONTROL_HEIGHT: f32 = 36.;
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

/// Unified region separation: every region is a softly floating card that
/// casts a small drop shadow toward the neighbouring region. Callers must
/// not add their own border on separated edges.
pub fn separate_region(region: gpui_kit::Div, edge: RegionEdge) -> gpui_kit::Div {
    region.shadow(region_shadows(edge))
}

fn region_shadows(edge: RegionEdge) -> Vec<gpui_kit::BoxShadow> {
    let (offset_x, offset_y) = match edge {
        RegionEdge::All => (0., 2.),
        RegionEdge::Left => (-2., 0.),
        RegionEdge::Right => (2., 0.),
        RegionEdge::Top => (0., -2.),
        RegionEdge::Bottom => (0., 2.),
    };
    vec![
        BoxShadow::new(px(offset_x), px(offset_y), rgba(shadow_overlay()).into())
            .blur_radius(px(6.))
            .spread_radius(px(-1.)),
    ]
}

/// Micro elevation for cards and floating controls (Fluent light feel):
/// a tight key shadow plus a soft ambient shadow reads as a lifted surface.
pub fn card_shadow() -> Vec<gpui_kit::BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(1.), rgba(shadow_overlay()).into()).blur_radius(px(2.)),
        BoxShadow::new(px(0.), px(2.), rgba(shadow_overlay()).into())
            .blur_radius(px(6.))
            .spread_radius(px(-1.)),
    ]
}

/// Compact icon-button variant of the micro elevation.
pub fn control_shadow() -> Vec<gpui_kit::BoxShadow> {
    vec![BoxShadow::new(px(0.), px(1.), rgba(shadow_overlay()).into()).blur_radius(px(2.))]
}

/// Elevation for top-level region cards in the main window composition.
pub fn region_shadow() -> Vec<gpui_kit::BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(1.), rgba(shadow_overlay()).into()).blur_radius(px(2.)),
        BoxShadow::new(px(0.), px(4.), rgba(shadow_overlay()).into())
            .blur_radius(px(10.))
            .spread_radius(px(-2.)),
    ]
}

/// Elevation for floating panel shells above the whole window.
pub fn floating_shadow() -> Vec<gpui_kit::BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(2.), rgba(shadow_overlay()).into()).blur_radius(px(4.)),
        BoxShadow::new(px(0.), px(8.), rgba(shadow_overlay()).into())
            .blur_radius(px(20.))
            .spread_radius(px(-3.)),
    ]
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
