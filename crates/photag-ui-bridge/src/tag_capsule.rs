//! Shared rendering for PhoTag tag labels across feature plugins.

use crate::theme;
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

/// Consistent tag marker for galleries, pickers, sidebars, and settings.
#[derive(Clone, Debug, Default)]
pub struct TagCapsule {
    name: String,
    color: Option<String>,
    icon: Option<String>,
    selected: bool,
    compact: bool,
}

impl TagCapsule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn color(mut self, color: Option<&str>) -> Self {
        self.color = color.map(str::to_owned);
        self
    }

    pub fn icon(mut self, icon: Option<&str>) -> Self {
        self.icon = icon.map(str::to_owned);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Use a denser capsule for overlays such as gallery thumbnails.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn render(self) -> Div {
        let tint = self
            .color
            .as_deref()
            .and_then(theme::parse_hex_color)
            .map(|[r, g, b]| (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b));
        let color = tint.unwrap_or(theme::TEXT_SECONDARY);
        let background = tint.map_or(theme::SURFACE_FAINT, |color| {
            theme::accent_alpha(color, if self.selected { 0.34 } else { 0.18 })
        });
        let border = tint.map_or(theme::SEPARATOR_LIGHT, |color| {
            theme::accent_alpha(color, if self.selected { 0.86 } else { 0.54 })
        });
        let (gap, height, padding, icon_size, font_size) = if self.compact {
            (2., 18., 4., 10., theme::FONT_SIZE_TINY)
        } else {
            (5., 24., 8., 13., theme::FONT_SIZE_SMALL)
        };
        div()
            .h_flex()
            .items_center()
            .gap(px(gap))
            .h(px(height))
            .min_w_0()
            .px(px(padding))
            .rounded_full()
            .border_1()
            .border_color(rgb(if self.selected {
                theme::accent()
            } else {
                border
            }))
            .bg(rgb(background))
            .text_size(px(font_size))
            .text_color(rgb(theme::TEXT_PRIMARY))
            .hover(|style| style.border_color(rgb(theme::accent())))
            .child(
                theme::small_icon(theme::tag_icon(self.icon.as_deref()))
                    .size(px(icon_size))
                    .flex_shrink_0()
                    .text_color(rgb(color)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.name),
            )
    }
}

/// Render a neutral tag capsule when a display name has no tag metadata.
pub fn name_only(name: impl Into<String>) -> Div {
    TagCapsule::new(name).icon(Some("Tag")).render()
}
