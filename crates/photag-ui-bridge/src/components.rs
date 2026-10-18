//! Shared workbench primitives. Callers own content, actions and lifecycle.
//! Import `WorkbenchStyle` for a Div whose ID/geometry the feature already owns.
use crate::theme;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{Sizable as _, Size};
use gpui_kit::*;

pub trait WorkbenchStyle: Sized {
    fn workbench_section(self) -> Self;
    fn workbench_card(self, selected: bool) -> Self;
    fn workbench_nav(self, selected: bool) -> Self;
}

impl<T: Styled + InteractiveElement> WorkbenchStyle for T {
    fn workbench_section(self) -> Self {
        self.bg(rgb(theme::SURFACE_PANEL))
            .rounded(px(theme::ROUND_LG))
            .border_1()
            .border_color(rgb(theme::SEPARATOR))
            .min_h_0()
    }

    fn workbench_card(self, selected: bool) -> Self {
        self.rounded(px(theme::ROUND_LG))
            .border_1()
            .border_color(rgb(if selected {
                theme::accent()
            } else {
                theme::SEPARATOR
            }))
            .bg(rgb(if selected {
                theme::selection_color()
            } else {
                theme::SURFACE_CARD
            }))
            .hover(move |style| {
                style.bg(rgb(if selected {
                    theme::selection_color()
                } else {
                    theme::SURFACE_ELEVATED
                }))
            })
    }

    fn workbench_nav(self, selected: bool) -> Self {
        self.h_flex()
            .items_center()
            .h(px(theme::NAV_ROW_HEIGHT))
            .px(px(theme::SPACE_3))
            .gap(px(theme::SPACE_3))
            .rounded(px(theme::ROUND_MD))
            .cursor_pointer()
            .bg(rgb(if selected {
                theme::selection_color()
            } else {
                theme::SURFACE_PANEL
            }))
            .text_color(rgb(if selected {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            }))
            .hover(move |style| {
                style.bg(rgb(if selected {
                    theme::selection_color()
                } else {
                    theme::SURFACE_CARD
                }))
            })
    }
}

pub fn section() -> Div {
    div().workbench_section()
}

/// Standard large action, including Kit-owned focus/hover/pressed/disabled states.
/// Use `.primary()`/`.danger()` on ordinary Kit buttons for other sizes.
pub fn action(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    primary: bool,
) -> Button {
    let button = Button::new(ElementId::Name(id.into()))
        .with_size(Size::Large)
        .label(label)
        .h(px(theme::ACTION_HEIGHT))
        .rounded(px(theme::ROUND_MD))
        .text_size(px(theme::FONT_SIZE_BUTTON));
    if primary { button.primary() } else { button }
}

pub fn icon_button(id: impl Into<SharedString>, name: gpui_kit::assets::IconName) -> Button {
    Button::new(ElementId::Name(id.into()))
        .ghost()
        .icon(theme::icon(name))
        .w(px(theme::ICON_BUTTON_SIZE))
        .h(px(theme::ICON_BUTTON_SIZE))
}

/// Native keyboard/focus behavior with the shared workbench navigation paint.
pub fn nav_button(id: impl Into<SharedString>, selected: bool) -> Button {
    Button::new(ElementId::Name(id.into()))
        .ghost()
        .w_full()
        .h(px(theme::NAV_ROW_HEIGHT))
        .justify_start()
        .rounded(px(theme::ROUND_MD))
        .text_size(px(theme::FONT_SIZE_BODY))
        .bg(rgba(if selected {
            (theme::selection_color() << 8) | 0xff
        } else {
            0
        }))
}

pub fn section_heading(label: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(theme::FONT_SIZE_HEADING))
        .font_semibold()
        .text_color(rgb(theme::TEXT_PRIMARY))
        .child(label.into())
}

/// Search fields share native focus/disabled states instead of custom outer paints.
pub fn search_input(state: &Entity<InputState>) -> Input {
    text_input(state)
        .w_full()
        .prefix(div().child(theme::icon(gpui_kit::assets::IconName::Search)))
        .cleanable(true)
}

pub fn text_input(state: &Entity<InputState>) -> Input {
    Input::new(state)
        .h(px(theme::SEARCH_FIELD_HEIGHT))
        .bg(rgb(theme::SURFACE_FAINT))
        .text_size(px(theme::FONT_SIZE_BODY))
}

pub fn textarea(state: &Entity<TextareaState>) -> Textarea {
    Textarea::new(state)
        .bg(rgb(theme::SURFACE_FAINT))
        .text_size(px(theme::FONT_SIZE_BODY))
}

/// Compact Explorer/sidebar frame shared by drives, quick access and favorite groups.
pub fn sidebar_section(body: Div) -> Div {
    theme::separate_region(
        body.v_flex()
            .w_full()
            .flex_shrink_0()
            .gap(px(2.))
            .p(px(6.))
            .rounded(px(theme::ROUND_MD))
            .bg(rgb(theme::SURFACE_REGION)),
        theme::RegionEdge::All,
    )
}

pub fn sidebar_header(
    label: impl Into<SharedString>,
    toggle: Button,
    menu: impl IntoElement,
    trailing: Vec<Button>,
) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap(px(2.))
        .pt(px(6.))
        .pb(px(2.))
        .child(toggle)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(11.5))
                .font_semibold()
                .text_color(rgb(theme::TEXT_MUTED))
                .text_ellipsis()
                .child(label.into()),
        )
        .child(menu)
        .children(trailing)
}
