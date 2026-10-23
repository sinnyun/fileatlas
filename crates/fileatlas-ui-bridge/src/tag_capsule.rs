//! Shared rendering for FileAtlas tag labels across feature plugins.

use crate::components::WorkbenchStyle as _;
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

    /// Use a denser capsule for overlays such as file_manager thumbnails.
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
        let colors = theme::tag_colors(tint, self.selected);
        let (gap, height, padding, icon_size) = if self.compact {
            (
                theme::space_compact(),
                theme::ui_height(18.),
                theme::space_compact(),
                10.,
            )
        } else {
            (
                theme::space_standard(),
                theme::ui_height(24.),
                theme::space_standard(),
                13.,
            )
        };
        let capsule = div()
            .h_flex()
            .items_center()
            .gap(px(gap))
            .h(px(height))
            .min_w_0()
            .px(px(padding))
            .rounded_full()
            .border_1()
            .border_color(rgb(colors.border))
            .bg(rgb(colors.background))
            .text_size(px(theme::FONT_SIZE_ASSISTANT))
            .line_height(px(theme::LINE_HEIGHT_ASSISTANT))
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(colors.foreground))
            .workbench_hover_accent_border()
            .child(
                theme::small_icon(theme::tag_icon(self.icon.as_deref()))
                    .size(px(icon_size))
                    .flex_shrink_0()
                    .text_color(rgb(colors.icon)),
            )
            .child(crate::components::observe_assistant_typography(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.name),
            ));
        crate::components::review_card(
            capsule,
            crate::geometry_audit::ComponentTokens {
                height: Some(theme::ui_height(height)),
                gap: Some(gap),
                padding_x: Some(padding),
                ..Default::default()
            },
        )
    }
}

/// Render a neutral tag capsule when a display name has no tag metadata.
pub fn name_only(name: impl Into<String>) -> Div {
    TagCapsule::new(name).icon(Some("Tag")).render()
}
