//! Shared filter popover controls.
use super::*;
use std::sync::{Arc, Mutex};
const POPOVER_ROW_HEIGHT: f32 = 30.;
const RATING_SLIDER_WIDTH: f32 = 224.;
const RATING_SLIDER_PAD: f32 = 9.;

/// Compact ghost trigger shared by every filter popover. Active filters use a
/// quiet accent wash while inactive controls stay visually light.
pub(super) fn chip(
    id: &'static str,
    icon: IconName,
    label: String,
    active: bool,
    clear: Option<AnyElement>,
) -> Button {
    let text_color = if active {
        theme::accent_text()
    } else {
        theme::text_secondary()
    };
    let trailing = clear.unwrap_or_else(|| {
        div()
            .ml(px(2.))
            .child(theme::small_icon(IconName::ChevronDown).text_color(rgb(theme::text_muted())))
            .into_any_element()
    });
    Button::new(id)
        .ghost()
        .compact()
        .h(px(theme::FILTER_CHIP_HEIGHT))
        .px(px(7.))
        .rounded(px(theme::ROUND_SM))
        .when(active, |button| button.bg(rgba(theme::fill_subtle())))
        .text_size(px(theme::FONT_SIZE_BUTTON))
        .text_color(rgb(text_color))
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(4.))
                .child(theme::small_icon(icon).text_color(rgb(if active {
                    theme::accent_text()
                } else {
                    theme::text_muted()
                })))
                .child(div().child(fileatlas_ui_bridge::i18n::tr(&label)))
                .child(trailing),
        )
}

/// The accent close icon of an active chip; stops the press so the popover does not
/// open, then clears that filter.
pub(super) fn clear_control(
    id: &'static str,
    clear: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .ml(px(2.))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(theme::small_icon(IconName::X).text_color(rgb(theme::accent_text())))
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            clear(event, window, cx);
        })
        .into_any_element()
}

pub(super) fn popover_title(title: &str) -> Div {
    div()
        .px(px(6.))
        .pb(px(6.))
        .text_size(px(theme::FONT_SIZE_SMALL))
        .text_color(rgb(theme::text_secondary()))
        .child(fileatlas_ui_bridge::i18n::tr(title))
}

pub(super) fn row_label(label: impl Into<String>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(theme::FONT_SIZE_BUTTON))
        .text_color(rgb(theme::text_primary()))
        .child(label.into())
}

pub(super) fn menu_row(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .h_flex()
        .items_center()
        .gap(px(8.))
        .h(px(POPOVER_ROW_HEIGHT))
        .px(px(6.))
        .rounded(px(theme::ROUND_SM))
        .cursor_pointer()
        .hover(|style| style.bg(rgba(theme::fill_hover())))
}

pub(super) fn empty_hint(message: &str) -> Div {
    div()
        .px(px(6.))
        .py(px(8.))
        .text_size(px(theme::FONT_SIZE_SMALL))
        .text_color(rgb(theme::text_muted()))
        .child(fileatlas_ui_bridge::i18n::tr(message))
}

/// Include/exclude segmented pair used by the tag and extension popovers.
pub(super) fn segment_pair(
    include_id: &'static str,
    exclude_id: &'static str,
    include_label: &'static str,
    exclude_label: &'static str,
    exclude_selected: bool,
    weak: WeakEntity<FileManagerView>,
    tags: bool,
    cx: &mut Context<gpui_kit::component::popover::PopoverState>,
) -> Div {
    let include_weak = weak.clone();
    let include = segment(include_id, include_label, !exclude_selected).on_click(cx.listener(
        move |_, _, _, cx| {
            let _ = include_weak.update(cx, |this, cx| {
                if tags {
                    this.tag_filter_exclude_mode = false;
                    cx.notify();
                } else {
                    this.set_extension_click_mode(false, cx);
                }
            });
            cx.notify();
        },
    ));
    let exclude = segment(exclude_id, exclude_label, exclude_selected).on_click(cx.listener(
        move |_, _, _, cx| {
            let _ = weak.update(cx, |this, cx| {
                if tags {
                    this.tag_filter_exclude_mode = true;
                    cx.notify();
                } else {
                    this.set_extension_click_mode(true, cx);
                }
            });
            cx.notify();
        },
    ));
    div()
        .h_flex()
        .w_full()
        .gap(px(4.))
        .pb(px(6.))
        .child(include)
        .child(exclude)
}

fn segment(id: &'static str, label: &'static str, selected: bool) -> Button {
    let text_color = if selected {
        theme::contrasting_text(theme::accent())
    } else {
        theme::text_secondary()
    };
    Button::new(id)
        .ghost()
        .compact()
        .flex_1()
        .h(px(26.))
        .rounded(px(theme::ROUND_SM))
        .text_size(px(theme::FONT_SIZE_SMALL))
        .text_color(rgb(text_color))
        .when(selected, |button| button.bg(rgb(theme::accent())))
        .label(fileatlas_ui_bridge::i18n::tr(label))
}

/// Weak-entity rating range slider for the rating popover (the filter-bar
/// slider cannot be reused because popover content has no file_manager context).
pub(super) fn rating_slider(
    weak: WeakEntity<FileManagerView>,
    popup: WeakEntity<gpui_kit::component::popover::PopoverState>,
    rating_min: i32,
    rating_max: i32,
) -> impl IntoElement {
    let track = RATING_SLIDER_WIDTH - RATING_SLIDER_PAD * 2.;
    let low = RATING_SLIDER_PAD + rating_min as f32 / 5. * track;
    let high = RATING_SLIDER_PAD + rating_max as f32 / 5. * track;
    let bounds = Arc::new(Mutex::new(Bounds::<Pixels>::default()));
    let layout_bounds = bounds.clone();
    let down_bounds = bounds.clone();
    let move_bounds = bounds;
    let down_weak = weak.clone();
    let move_weak = weak.clone();
    let up_weak = weak;
    let down_popup = popup.clone();
    div()
        .id("file_manager-rating-popover-slider")
        .relative()
        .w(px(RATING_SLIDER_WIDTH))
        .flex_shrink_0()
        .h(px(22.))
        .cursor_pointer()
        .child(
            canvas(
                move |bounds, _, _| {
                    *layout_bounds.lock().unwrap() = bounds;
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
        .child(
            div()
                .absolute()
                .top(px(9.5))
                .left(px(RATING_SLIDER_PAD))
                .w(px(track))
                .h(px(3.))
                .rounded_full()
                .bg(rgba(theme::track_highlight())),
        )
        .child(
            div()
                .absolute()
                .top(px(9.5))
                .left(px(low))
                .w(px((high - low).max(0.)))
                .h(px(3.))
                .rounded_full()
                .bg(rgb(theme::accent())),
        )
        .child(slider_thumb(low))
        .child(slider_thumb(high))
        .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
            let x = f32::from(event.position.x - down_bounds.lock().unwrap().origin.x);
            let target = if (x - low).abs() < (x - high).abs() {
                sliders::SliderDrag::RatingMin
            } else {
                sliders::SliderDrag::RatingMax
            };
            let _ = down_weak.update(cx, |this, cx| {
                this.slider_drag = Some(target);
                this.change_slider(x, RATING_SLIDER_PAD, track, cx);
            });
            let _ = down_popup.update(cx, |_, cx| cx.notify());
        })
        .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
            if !event.dragging() {
                return;
            }
            let x = f32::from(event.position.x - move_bounds.lock().unwrap().origin.x);
            let _ = move_weak.update(cx, |this, cx| {
                if this.slider_drag.is_some() {
                    this.change_slider(x, RATING_SLIDER_PAD, track, cx);
                }
            });
            let _ = popup.update(cx, |_, cx| cx.notify());
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            let _ = up_weak.update(cx, |this, _| this.slider_drag = None);
        })
}

fn slider_thumb(x: f32) -> Div {
    div()
        .absolute()
        .left(px(x - 8.))
        .top(px(3.))
        .size(px(16.))
        .rounded_full()
        .bg(rgba(theme::track_shadow()))
        .child(
            div()
                .absolute()
                .inset(px(1.5))
                .rounded_full()
                .bg(rgb(theme::surface_elevated())),
        )
        .child(
            div()
                .absolute()
                .inset(px(4.8))
                .rounded_full()
                .bg(rgb(theme::accent())),
        )
}

/// Both native triggers and plugin MCP control the same Popover open state.
pub(super) fn controlled_popover(
    view: &FileManagerView,
    weak: WeakEntity<FileManagerView>,
    id: &'static str,
    name: &'static str,
) -> Popover {
    Popover::new(id)
        .open(view.surfaces.value("file_manager.popover").as_deref() == Some(name))
        .on_open_change(move |open, _, cx| {
            let _ = weak.update(cx, |this, cx| {
                let value = if *open { name } else { "" };
                if let Err(error) = this.surfaces.set_value("file_manager.popover", value) {
                    tracing::error!("popover state failed: {error}");
                }
                cx.notify();
            });
        })
}
