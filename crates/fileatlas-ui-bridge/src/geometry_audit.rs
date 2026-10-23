//! Read-only snapshots of resolved typography bounds during the current GPUI frame.
//! This stays in the UI bridge so feature plugins share one measurement boundary.

use gpui_kit::{Bounds, Pixels, Window};
use serde::Serialize;
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Debug, Serialize)]
pub struct ElementGeometry {
    pub window: String,
    pub tier: String,
    pub component_id: Option<String>,
    pub source_file: String,
    pub source_line: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub height_token: Option<f32>,
    pub min_width_token: Option<f32>,
    pub gap_token: Option<f32>,
    pub padding_x_token: Option<f32>,
    pub padding_y_token: Option<f32>,
    pub font_size: f32,
    pub line_height: f32,
    pub font_weight: String,
    pub font_family: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ComponentTokens {
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub gap: Option<f32>,
    pub padding_x: Option<f32>,
    pub padding_y: Option<f32>,
}

#[derive(Default)]
struct Snapshot {
    generation: u64,
    entries: Vec<ElementGeometry>,
}

fn snapshots() -> &'static RwLock<std::collections::BTreeMap<String, Snapshot>> {
    static SNAPSHOTS: OnceLock<RwLock<std::collections::BTreeMap<String, Snapshot>>> =
        OnceLock::new();
    SNAPSHOTS.get_or_init(Default::default)
}

/// Mark the start of a root view render. Canvas observers in that window then
/// replace this frame's values instead of returning stale entries from prior frames.
pub fn begin_window_frame(window: &str) {
    let Ok(mut all) = snapshots().write() else {
        return;
    };
    let frame = all.entry(window.to_owned()).or_default();
    frame.generation = frame.generation.wrapping_add(1);
    frame.entries.clear();
}

pub fn record(
    bounds: Bounds<Pixels>,
    window: &mut Window,
    tier: &str,
    file: &str,
    line: u32,
    component_id: Option<&str>,
    tokens: ComponentTokens,
) {
    let handle = window.window_handle();
    let window_id = format!("{:?}", handle.window_id());
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = style
        .line_height
        .to_pixels(style.font_size, window.rem_size());
    let item = ElementGeometry {
        window: window_id.clone(),
        tier: tier.to_owned(),
        component_id: component_id.map(str::to_owned),
        source_file: file.to_owned(),
        source_line: line,
        // Audit outlines are painted over the measured box and do not affect
        // layout, so canvas bounds already match the component's outer box.
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        width: f32::from(bounds.size.width),
        height: f32::from(bounds.size.height),
        height_token: tokens.height,
        min_width_token: tokens.min_width,
        gap_token: tokens.gap,
        padding_x_token: tokens.padding_x,
        padding_y_token: tokens.padding_y,
        font_size: f32::from(font_size),
        line_height: f32::from(line_height),
        font_weight: format!("{:?}", style.font_weight),
        font_family: style.font_family.to_string(),
    };
    if let Ok(mut all) = snapshots().write() {
        let frame = all.entry(window_id).or_default();
        if frame.entries.len() < 20_000 {
            frame.entries.push(item);
        }
    }
}

pub fn snapshot(window: Option<&str>, tier: Option<&str>) -> Vec<ElementGeometry> {
    let Ok(all) = snapshots().read() else {
        return Vec::new();
    };
    all.iter()
        .filter(|(id, _)| window.is_none_or(|window| id.as_str() == window))
        .flat_map(|(_, frame)| frame.entries.iter())
        .filter(|item| tier.is_none_or(|tier| item.tier == tier))
        .cloned()
        .collect()
}
