//! Foreground/background pairs for content-dependent paints.
use super::*;

/// Choose an existing native foreground role for user-colored content.
pub fn contrasting_text(background: u32) -> u32 {
    let colors = native_colors();
    let candidates = [
        colors.foreground,
        colors.primary_foreground,
        colors.popover_foreground,
        colors.danger_foreground,
        colors.warning_foreground,
    ];
    candidates
        .into_iter()
        .map(packed)
        .max_by(|a, b| contrast_ratio(*a, background).total_cmp(&contrast_ratio(*b, background)))
        .unwrap()
}

/// Keep semantic colored text readable on raised cards, badges and controls.
pub fn readable_foreground(color: u32) -> u32 {
    readable_foreground_on(color, surface_elevated())
}

/// Adjust a colored foreground for the actual opaque surface behind it.
pub fn readable_foreground_on(color: u32, background: u32) -> u32 {
    if contrast_ratio(color, background) >= 4.5 {
        return color;
    }
    contrasting_text(background)
}

pub fn contrast_ratio(foreground: u32, background: u32) -> f32 {
    fn luminance(color: u32) -> f32 {
        let [r, g, b] = channels(color);
        let linear = |channel: u8| {
            let c = f32::from(channel) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    }
    let (light, dark) = (luminance(foreground), luminance(background));
    (light.max(dark) + 0.05) / (light.min(dark) + 0.05)
}

/// Stable low-saturation file-type colors, with foreground paired to the actual fill.
pub fn file_format_colors(extension: &str) -> (u32, u32) {
    let hash = extension.bytes().fold(2166136261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    });
    let colors = native_colors();
    let pairs = [
        (colors.info, colors.info_foreground),
        (colors.primary, colors.primary_foreground),
        (colors.warning, colors.warning_foreground),
        (colors.success, colors.success_foreground),
        (colors.danger, colors.danger_foreground),
    ];
    let (background, foreground) = pairs[hash as usize % pairs.len()];
    (
        packed(background),
        readable_foreground_on(packed(foreground), packed(background)),
    )
}

/// An opaque capsule surface and its paired foregrounds, shared by all tag placements.
pub struct TagColors {
    pub background: u32,
    pub foreground: u32,
    pub icon: u32,
    pub border: u32,
}
pub fn tag_colors(tint: Option<u32>, selected: bool) -> TagColors {
    let background = if selected {
        selection_color()
    } else {
        surface_faint()
    };
    TagColors {
        background,
        foreground: readable_foreground_on(text_primary(), background),
        icon: readable_foreground_on(tint.unwrap_or(text_secondary()), background),
        border: if selected { accent() } else { separator() },
    }
}
