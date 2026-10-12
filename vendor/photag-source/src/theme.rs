#![allow(dead_code)]
//! Centralised design tokens. Other modules pull colors / spacing from here
//! instead of hard-coding so the look stays consistent across the gallery,
//! sidebar, filter bar, preview, and modals.
//!
//! Design language: "quiet chrome, content first". The window chrome
//! (command bar, sidebar, status bar) sits directly on the base material —
//! no fills, no outlines — while the gallery is the single elevated card.
//! Controls are borderless ghosts that reveal themselves on hover, the
//! accent is used only for selection/focus, and iconography comes from
//! Segoe MDL2 Assets (real Fluent glyphs) rather than emoji.

use egui::{Color32, Rounding, Stroke};

// ── Surfaces (low → high elevation) ──────────────────────────────────────
// Pure neutrals — zero color cast, so nothing competes with photo content.

/// Window base — the material the chrome (command bar, sidebar, status bar)
/// sits on directly. The Mica/Acrylic backdrop shows here when active.
pub const SURFACE_BASE: Color32 = Color32::from_rgb(25, 25, 25);
/// The single elevated content card (the gallery), plus preview chrome.
pub const SURFACE_PANEL: Color32 = Color32::from_rgb(37, 37, 37);
/// Placeholder thumbnails, inset wells, and other raised fills.
pub const SURFACE_ELEVATED: Color32 = Color32::from_rgb(46, 46, 46);
/// Modal windows / flyouts.
pub const SURFACE_MODAL: Color32 = Color32::from_rgb(43, 43, 43);
/// Subtle alternate-row / faint background tint.
pub const SURFACE_FAINT: Color32 = Color32::from_rgb(31, 31, 31);

/// Interactive-control fills. Translucent white so they read correctly on
/// both the base chrome and the elevated gallery card.
pub const CONTROL_HOVER:  Color32 = Color32::from_rgba_premultiplied(22, 22, 22, 22);
pub const CONTROL_ACTIVE: Color32 = Color32::from_rgba_premultiplied(12, 12, 12, 12);

// ── Text ─────────────────────────────────────────────────────────────────

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(235, 235, 235);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 160);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(112, 112, 112);

// ── Accent (user-selectable preset) ─────────────────────────────────────
// One muted accent used sparingly: selection, focus, active toggles.
// Never used as widget chrome.
//
// Stored in an atomic so the user can swap presets at runtime via the
// Appearance settings tab without restarting the app.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Named preset shown in the Appearance picker.
pub struct AccentPreset {
    pub name: &'static str,
    pub color: Color32,
}

pub const ACCENT_PRESETS: &[AccentPreset] = &[
    AccentPreset { name: "Slate Blue",   color: Color32::from_rgb(0x5A, 0x7F, 0xA0) },
    AccentPreset { name: "Dusty Teal",   color: Color32::from_rgb(0x5A, 0x8A, 0x88) },
    AccentPreset { name: "Muted Copper", color: Color32::from_rgb(0xA0, 0x65, 0x4A) },
    AccentPreset { name: "Dusty Rose",   color: Color32::from_rgb(0xA0, 0x74, 0x78) },
    AccentPreset { name: "Sage Green",   color: Color32::from_rgb(0x6A, 0x9A, 0x78) },
    AccentPreset { name: "Dusty Mauve",  color: Color32::from_rgb(0x8A, 0x70, 0x98) },
    AccentPreset { name: "Warm Taupe",   color: Color32::from_rgb(0x7A, 0x6B, 0x5E) },
];

/// Sentinel accent name meaning "track the live Windows system accent".
pub const SYSTEM_ACCENT_NAME: &str = "System";

/// Default accent for a fresh install — match the OS accent color.
pub const DEFAULT_ACCENT_NAME: &str = SYSTEM_ACCENT_NAME;

/// Concrete preset used when the system accent can't be read or the stored
/// name is unknown.
pub const FALLBACK_ACCENT_NAME: &str = "Slate Blue";

/// Look up a manual preset by name. Falls back to `FALLBACK_ACCENT_NAME`.
pub fn preset_by_name(name: &str) -> &'static AccentPreset {
    ACCENT_PRESETS.iter().find(|p| p.name == name)
        .unwrap_or_else(|| ACCENT_PRESETS.iter().find(|p| p.name == FALLBACK_ACCENT_NAME).unwrap())
}

/// Resolve an accent name — either a manual preset or the `System` sentinel
/// (which reads the live Windows accent) — to a concrete color.
pub fn resolve_accent_color(name: &str) -> Color32 {
    if name == SYSTEM_ACCENT_NAME {
        crate::winfx::system_accent()
            .map(normalize_accent)
            .unwrap_or_else(|| preset_by_name(FALLBACK_ACCENT_NAME).color)
    } else {
        preset_by_name(name).color
    }
}

/// Guard against system accents too dark to work as a selection color on a
/// dark UI (e.g. Windows' grey/charcoal accents). Preserves the hue but
/// lifts luminance to a floor where rings, badges, and highlights stay
/// legible. Colors already bright enough pass through untouched.
fn normalize_accent(c: Color32) -> Color32 {
    const MIN_LUMA: f32 = 112.0;
    let luma = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if luma >= MIN_LUMA { return c; }
    if luma < 1.0 {
        // Pure black — no hue to preserve; use the fallback preset.
        return preset_by_name(FALLBACK_ACCENT_NAME).color;
    }
    let f = MIN_LUMA / luma;
    Color32::from_rgb(
        (c.r() as f32 * f).min(255.0) as u8,
        (c.g() as f32 * f).min(255.0) as u8,
        (c.b() as f32 * f).min(255.0) as u8,
    )
}

/// Packed [r, g, b, _] of the current accent — read on every paint.
static ACCENT_RAW: AtomicU32 = AtomicU32::new(0x5A7FA0FF);

fn pack(c: Color32) -> u32 {
    u32::from_be_bytes([c.r(), c.g(), c.b(), 0xFF])
}
fn unpack(v: u32) -> Color32 {
    let [r, g, b, _] = v.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

/// Current accent color. Used for selection, focus, and active indicators.
pub fn accent() -> Color32 { unpack(ACCENT_RAW.load(Ordering::Relaxed)) }

/// Brighter accent for hover.
pub fn accent_hover() -> Color32 {
    let c = accent();
    Color32::from_rgb(
        c.r().saturating_add(24),
        c.g().saturating_add(24),
        c.b().saturating_add(24),
    )
}

/// Low-alpha accent fill — active toggle pills + TextEdit text selection.
pub fn accent_soft() -> Color32 {
    let c = accent();
    Color32::from_rgba_premultiplied(
        ((c.r() as u32 * 70) / 255) as u8,
        ((c.g() as u32 * 70) / 255) as u8,
        ((c.b() as u32 * 70) / 255) as u8,
        70,
    )
}

/// Accent at an arbitrary strength (0.0–1.0) — premultiplied, safe to layer.
pub fn accent_alpha(f: f32) -> Color32 {
    accent().gamma_multiply(f.clamp(0.0, 1.0))
}

/// Update the accent color and re-apply visuals so live widgets pick up the
/// new selection-fill / hyperlink color on the next frame.
pub fn set_accent(ctx: &egui::Context, color: Color32) {
    ACCENT_RAW.store(pack(color), Ordering::Relaxed);
    apply(ctx);
}

/// Width of the left-edge selection bar (B2 selection style).
pub const SELECTION_BAR_W: f32 = 3.0;

// ── Semantic ─────────────────────────────────────────────────────────────

pub const DANGER: Color32 = Color32::from_rgb(232, 96, 96);
pub const WARNING: Color32 = Color32::from_rgb(232, 168, 96);
/// Rating-star gold.
pub const STAR: Color32 = Color32::from_rgb(240, 200, 80);

// ── Strokes & borders ────────────────────────────────────────────────────
// No light outlines anywhere. Surfaces separate by fill contrast and
// shadow; controls by fill alone. Where a floating surface genuinely needs
// a crisp edge against unpredictable content underneath (a menu over the
// gallery, an info panel over a photo), the edge is *dark* — it reads as
// depth, the way a real shadowed edge would, instead of a drawn-on
// wireframe.

/// Separator groove inside modals and panels.
pub const HAIRLINE: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 64);
/// Edge for floating surfaces — windows, menus, flyouts, info panels.
pub const BORDER: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 130);

// ── Geometry ─────────────────────────────────────────────────────────────
// Win11 uses a 4px control corner radius and an 8px overlay/dialog radius.

pub const ROUND_SM: f32 = 4.0;
pub const ROUND_MD: f32 = 6.0;
pub const ROUND_LG: f32 = 8.0;
/// The elevated content card.
pub const ROUND_XL: f32 = 10.0;
pub const ROUND_PILL: f32 = 999.0;

/// Corner radius for gallery thumbnails.
pub const THUMB_ROUND: f32 = 7.0;

pub fn rounding(r: f32) -> Rounding { Rounding::same(r) }

// ── Typography ───────────────────────────────────────────────────────────
// Size scale: small / body / button / heading / title. Heading + title use
// Segoe UI Semibold (a real weight, not a fake bold) when available.

pub const FONT_SIZE_TINY:    f32 = 10.5;
pub const FONT_SIZE_SMALL:   f32 = 12.0;
pub const FONT_SIZE_BODY:    f32 = 13.5;
pub const FONT_SIZE_BUTTON:  f32 = 13.0;
pub const FONT_SIZE_HEADING: f32 = 17.0;
/// Month headers / large section titles.
pub const FONT_SIZE_TITLE:   f32 = 19.0;

/// Set once fonts are installed — whether the Semibold face was found.
static SEMIBOLD_OK: AtomicBool = AtomicBool::new(false);
/// Set once fonts are installed — whether Segoe MDL2 Assets was found.
static ICONS_OK: AtomicBool = AtomicBool::new(false);

pub const FAMILY_SEMIBOLD: &str = "semibold";
pub const FAMILY_ICONS: &str = "fluent-icons";

/// FontFamily for emphasized text — Segoe UI Semibold when installed,
/// regular proportional otherwise.
pub fn semibold_family() -> egui::FontFamily {
    if SEMIBOLD_OK.load(Ordering::Relaxed) {
        egui::FontFamily::Name(FAMILY_SEMIBOLD.into())
    } else {
        egui::FontFamily::Proportional
    }
}

/// FontId for emphasized text at `size`.
pub fn semibold(size: f32) -> egui::FontId {
    egui::FontId::new(size, semibold_family())
}

/// Whether real Fluent icon glyphs are available (Segoe MDL2 Assets).
pub fn icons_available() -> bool { ICONS_OK.load(Ordering::Relaxed) }

/// FontId for icon glyphs at `size`.
pub fn icon_font(size: f32) -> egui::FontId {
    if icons_available() {
        egui::FontId::new(size, egui::FontFamily::Name(FAMILY_ICONS.into()))
    } else {
        egui::FontId::new(size, egui::FontFamily::Proportional)
    }
}

// ── Fluent icon glyphs (Segoe MDL2 Assets) ──────────────────────────────
// Documented, Win10-stable codepoints only.

pub const ICON_SEARCH: &str        = "\u{E721}";
pub const ICON_SETTINGS: &str      = "\u{E713}";
pub const ICON_FILTER: &str        = "\u{E71C}";
pub const ICON_SORT: &str          = "\u{E8CB}";
pub const ICON_CALENDAR: &str      = "\u{E787}";
pub const ICON_FOLDER: &str        = "\u{E8B7}";
pub const ICON_FOLDER_OPEN: &str   = "\u{E838}";
pub const ICON_TAG: &str           = "\u{E8EC}";
pub const ICON_CHEVRON_DOWN: &str  = "\u{E70D}";
pub const ICON_CHEVRON_RIGHT: &str = "\u{E76C}";
pub const ICON_PLAY: &str          = "\u{E768}";
pub const ICON_CHECK: &str         = "\u{E73E}";
pub const ICON_STAR: &str          = "\u{E735}";
pub const ICON_CLOSE: &str         = "\u{E711}";
pub const ICON_ADD: &str           = "\u{E710}";
pub const ICON_PICTURE: &str       = "\u{E8B9}";
pub const ICON_VIDEO: &str         = "\u{E714}";
pub const ICON_AUDIO: &str         = "\u{E767}";
pub const ICON_DELETE: &str        = "\u{E74D}";
pub const ICON_ZOOM: &str          = "\u{E8A3}";
pub const ICON_VIEW: &str          = "\u{E7B3}";
pub const ICON_MORE: &str          = "\u{E712}";
pub const ICON_LIBRARY: &str       = "\u{E8A9}";

/// One icon glyph as RichText in the Fluent icon font. Falls back to the
/// supplied emoji when MDL2 isn't installed.
pub fn icon(glyph: &str, fallback: &str, size: f32, color: Color32) -> egui::RichText {
    if icons_available() {
        egui::RichText::new(glyph).font(icon_font(size)).color(color)
    } else {
        egui::RichText::new(fallback).size(size).color(color)
    }
}

/// LayoutJob: icon glyph + gap + text, vertically aligned. Used for buttons
/// and rows that pair a Fluent glyph with a label.
pub fn icon_label(
    glyph: &str,
    fallback: &str,
    text: &str,
    icon_size: f32,
    text_font: egui::FontId,
    icon_color: Color32,
    text_color: Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let (icon_str, icon_font_id) = if icons_available() {
        (glyph, icon_font(icon_size))
    } else {
        (fallback, egui::FontId::proportional(icon_size))
    };
    job.append(icon_str, 0.0, egui::TextFormat {
        font_id: icon_font_id,
        color: icon_color,
        valign: egui::Align::Center,
        ..Default::default()
    });
    job.append(text, 7.0, egui::TextFormat {
        font_id: text_font,
        color: text_color,
        valign: egui::Align::Center,
        ..Default::default()
    });
    job
}

/// Load Segoe UI as the primary UI font, Segoe UI Semibold as the heading
/// face, and Segoe MDL2 Assets for Fluent icon glyphs (with Segoe UI
/// Symbol & Emoji as pictograph fallbacks). Zero binary cost — all system
/// fonts read from C:\Windows\Fonts at startup.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Primary UI font candidates. Each is prepended to the family chain, so
    // the LAST one present wins: prefer Win11's Segoe UI Variable, falling
    // back to Win10's Segoe UI when the variable font isn't installed.
    let ui_candidates = [
        (r"C:\Windows\Fonts\segoeui.ttf",  "segoeui"),  // Segoe UI (Win10)
        (r"C:\Windows\Fonts\SegUIVar.ttf", "seguivar"), // Segoe UI Variable (Win11)
    ];
    // Fallback fonts for pictographs / emoji.
    let fallback_candidates = [
        (r"C:\Windows\Fonts\seguisym.ttf", "seguisym"), // Segoe UI Symbol
        (r"C:\Windows\Fonts\seguiemj.ttf", "seguiemj"), // Segoe UI Emoji
    ];

    // Insert the primary font at the FRONT of both Proportional and
    // Monospace family chains so it wins for Latin glyphs.
    for (path, name) in ui_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.to_string(), egui::FontData::from_owned(bytes));
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(fam).or_default().insert(0, name.to_string());
            }
        }
    }

    // Append fallbacks at the END so they only fire for missing glyphs.
    for (path, name) in fallback_candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.to_string(), egui::FontData::from_owned(bytes));
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(fam).or_default().push(name.to_string());
            }
        }
    }

    // Segoe UI Semibold → named family for headings / titles. Chain the
    // pictograph fallbacks behind it so mixed strings still render.
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\seguisb.ttf") {
        fonts.font_data.insert("seguisb".to_string(), egui::FontData::from_owned(bytes));
        let mut chain = vec!["seguisb".to_string()];
        for name in ["seguisym", "seguiemj"] {
            if fonts.font_data.contains_key(name) { chain.push(name.to_string()); }
        }
        fonts.families.insert(egui::FontFamily::Name(FAMILY_SEMIBOLD.into()), chain);
        SEMIBOLD_OK.store(true, Ordering::Relaxed);
    }

    // Segoe MDL2 Assets → named family for Fluent icon glyphs.
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segmdl2.ttf") {
        fonts.font_data.insert("segmdl2".to_string(), egui::FontData::from_owned(bytes));
        fonts.families.insert(
            egui::FontFamily::Name(FAMILY_ICONS.into()),
            vec!["segmdl2".to_string()],
        );
        ICONS_OK.store(true, Ordering::Relaxed);
    }

    ctx.set_fonts(fonts);
}

/// Whether a translucent window backdrop (Win11 Mica / Win10 Acrylic) is
/// active. When set, panel & surface fills gain a little transparency so the
/// material shows through the app chrome.
static BACKDROP_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn backdrop_active() -> bool { BACKDROP_ACTIVE.load(Ordering::Relaxed) }

/// Fill for the elevated gallery card. Goes slightly translucent when a
/// Mica/Acrylic backdrop is active so the material reads through.
pub fn card_fill() -> Color32 {
    if backdrop_active() { with_alpha(SURFACE_PANEL, 0xE8) } else { SURFACE_PANEL }
}

/// Toggle translucency mode and re-apply visuals so live panels pick up the
/// new fills on the next frame. Called once the backdrop is applied at startup.
pub fn set_backdrop_active(ctx: &egui::Context, active: bool) {
    BACKDROP_ACTIVE.store(active, Ordering::Relaxed);
    apply(ctx);
}

/// Premultiplied copy of `c` at alpha `a` (for translucent surface fills).
fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Apply the theme to an egui context. Call once at startup (and again when
/// the backdrop / accent changes).
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    let translucent = backdrop_active();

    // Surface fills. When a backdrop is active they go slightly translucent
    // so the Mica/Acrylic material reads through the chrome; otherwise they
    // stay fully opaque.
    let (base, modal) = if translucent {
        (with_alpha(SURFACE_BASE, 0xDE), with_alpha(SURFACE_MODAL, 0xF4))
    } else {
        (SURFACE_BASE, SURFACE_MODAL)
    };

    visuals.panel_fill       = base;
    visuals.window_fill      = modal;
    // Inset wells (text fields, sunken lists) — darker than any surface.
    visuals.extreme_bg_color = Color32::from_rgb(19, 19, 19);
    visuals.faint_bg_color   = Color32::from_rgba_premultiplied(8, 8, 8, 8);

    visuals.override_text_color = Some(TEXT_PRIMARY);

    // Selection (text highlight in TextEdit + active toggle-pill fill).
    visuals.selection.bg_fill = accent_soft();
    visuals.selection.stroke  = Stroke::NONE;

    // Non-interactive chrome separates by fill steps only — no borders.
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, HAIRLINE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);

    // Interactive widgets — quiet ghosts: translucent-white fills that work
    // on any surface, with NO outlines at all — the fill alone carries the
    // affordance. Hover brightens, press dims (the Fluent pressed behavior).
    let ctrl = rounding(ROUND_SM + 1.0);

    let ghost      = Color32::from_rgba_premultiplied(16, 16, 16, 16);
    let ghost_hov  = Color32::from_rgba_premultiplied(28, 28, 28, 28);
    let ghost_down = Color32::from_rgba_premultiplied(10, 10, 10, 10);

    // egui splits these two: buttons paint with `weak_bg_fill`, while
    // checkboxes, radios, slider rails and scrollbar handles paint with
    // `bg_fill`. Keeping them apart is what lets buttons stay ghosted
    // while an *unchecked* checkbox still reads as a real box — solved
    // with a fill, never with an outline.
    let box_fill      = Color32::from_rgb(74, 74, 74);
    let box_fill_hov  = Color32::from_rgb(92, 92, 92);
    let box_fill_down = Color32::from_rgb(104, 104, 104);

    visuals.widgets.inactive.bg_fill      = box_fill;
    visuals.widgets.inactive.weak_bg_fill = ghost;
    visuals.widgets.inactive.bg_stroke    = Stroke::NONE;
    visuals.widgets.inactive.fg_stroke    = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding     = ctrl;

    visuals.widgets.hovered.bg_fill       = box_fill_hov;
    visuals.widgets.hovered.weak_bg_fill  = ghost_hov;
    visuals.widgets.hovered.bg_stroke     = Stroke::NONE;
    visuals.widgets.hovered.fg_stroke     = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.rounding      = ctrl;

    visuals.widgets.active.bg_fill        = box_fill_down;
    visuals.widgets.active.weak_bg_fill   = ghost_down;
    visuals.widgets.active.bg_stroke      = Stroke::NONE;
    visuals.widgets.active.fg_stroke      = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.rounding       = ctrl;

    visuals.widgets.open.bg_fill          = box_fill_hov;
    visuals.widgets.open.weak_bg_fill     = ghost_hov;
    visuals.widgets.open.bg_stroke        = Stroke::NONE;
    visuals.widgets.open.fg_stroke        = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.open.rounding         = ctrl;

    // Window / popup chrome — Fluent flyout: rounded, a faint border, and a
    // soft drop shadow for elevation.
    visuals.window_rounding   = rounding(ROUND_LG);
    visuals.menu_rounding     = rounding(ROUND_LG);
    visuals.window_stroke     = Stroke::new(1.0, BORDER);
    visuals.popup_shadow      = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(140),
    };
    visuals.window_shadow     = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 12.0),
        blur: 36.0,
        spread: 0.0,
        color: Color32::from_black_alpha(160),
    };

    // Slider: round thumb, accent-filled trailing track.
    visuals.handle_shape = egui::style::HandleShape::Circle;
    visuals.slider_trailing_fill = true;

    visuals.hyperlink_color = accent_hover();

    ctx.set_visuals(visuals);

    // Spacing — a touch more breathing room than egui defaults.
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing      = egui::vec2(8.0, 6.0);
    style.spacing.button_padding    = egui::vec2(12.0, 5.0);
    style.spacing.menu_margin       = egui::Margin::symmetric(10.0, 8.0);
    style.spacing.window_margin     = egui::Margin::same(14.0);
    style.spacing.scroll.bar_width  = 7.0;
    style.spacing.scroll.floating   = true;
    // Make the side-panel resize handle easier to grab.
    style.interaction.resize_grab_radius_side = 8.0;

    // Typography — clear size hierarchy; headings render in Segoe UI
    // Semibold (a genuine weight) when the face is installed.
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading,  FontId::new(FONT_SIZE_HEADING, semibold_family())),
        (TextStyle::Body,     FontId::new(FONT_SIZE_BODY,    FontFamily::Proportional)),
        (TextStyle::Button,   FontId::new(FONT_SIZE_BUTTON,  FontFamily::Proportional)),
        (TextStyle::Small,    FontId::new(FONT_SIZE_SMALL,   FontFamily::Proportional)),
        (TextStyle::Monospace,FontId::new(FONT_SIZE_BODY,    FontFamily::Monospace)),
    ].into();

    ctx.set_style(style);
}

/// Windows 11 NavigationView-style selection: paint a short, rounded accent
/// pill on the left edge of an already-laid-out row. Call after rendering the
/// selectable row when `is_selected` is true. The pill is vertically centered
/// and spans ~half the row height, with fully rounded ends — the recognizable
/// Fluent "this item is selected" indicator.
pub fn paint_selection_bar(ui: &egui::Ui, row_rect: egui::Rect, is_selected: bool) {
    if !is_selected { return; }
    let panel_left = ui.clip_rect().left();
    let h = (row_rect.height() * 0.55).clamp(10.0, 18.0);
    let cy = row_rect.center().y;
    let x = panel_left + 2.0;
    let bar = egui::Rect::from_min_max(
        egui::pos2(x, cy - h / 2.0),
        egui::pos2(x + SELECTION_BAR_W, cy + h / 2.0),
    );
    ui.painter().rect_filled(bar, rounding(SELECTION_BAR_W / 2.0), accent());
}

/// Paint a rounded rect filled with a texture, center-cropped to cover the
/// rect (the CSS `object-fit: cover` behavior). `tint` multiplies the
/// texture — pass WHITE for normal display, or WHITE.gamma_multiply(t) for
/// a fade-in.
pub fn paint_cover_image(
    painter: &egui::Painter,
    rect: egui::Rect,
    tex_size: egui::Vec2,
    tex_id: egui::TextureId,
    round: Rounding,
    tint: Color32,
) {
    let uv = cover_uv(tex_size, rect.size());
    painter.add(egui::epaint::RectShape {
        rect,
        rounding: round,
        fill: tint,
        stroke: Stroke::NONE,
        blur_width: 0.0,
        fill_texture_id: tex_id,
        uv,
    });
}

/// UV rect that center-crops a texture of `tex` size to fill `cell`.
pub fn cover_uv(tex: egui::Vec2, cell: egui::Vec2) -> egui::Rect {
    if tex.x <= 0.0 || tex.y <= 0.0 || cell.x <= 0.0 || cell.y <= 0.0 {
        return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    }
    let tex_aspect = tex.x / tex.y;
    let cell_aspect = cell.x / cell.y;
    if tex_aspect > cell_aspect {
        // Texture is wider — crop left/right.
        let span = cell_aspect / tex_aspect;
        let x0 = (1.0 - span) * 0.5;
        egui::Rect::from_min_max(egui::pos2(x0, 0.0), egui::pos2(x0 + span, 1.0))
    } else {
        // Texture is taller — crop top/bottom.
        let span = tex_aspect / cell_aspect;
        let y0 = (1.0 - span) * 0.5;
        egui::Rect::from_min_max(egui::pos2(0.0, y0), egui::pos2(1.0, y0 + span))
    }
}

/// Soft blurred drop shadow under a rounded rect.
pub fn paint_soft_shadow(
    painter: &egui::Painter,
    rect: egui::Rect,
    round: Rounding,
    offset_y: f32,
    blur: f32,
    color: Color32,
) {
    painter.add(egui::epaint::RectShape {
        rect: rect.translate(egui::vec2(0.0, offset_y)),
        rounding: round,
        fill: color,
        stroke: Stroke::NONE,
        blur_width: blur,
        fill_texture_id: egui::TextureId::default(),
        uv: egui::Rect::ZERO,
    });
}
