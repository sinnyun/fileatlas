//! Shared filter popover controls.
use super::*;
use gpui_kit::component::button::Button;
use std::sync::{Arc, Mutex};
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
    let trailing = clear.unwrap_or_else(|| {
        div()
            .ml(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(
                theme::small_icon(IconName::ChevronDown)
                    .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted),
            )
            .into_any_element()
    });
    fileatlas_ui_bridge::components::button(id)
        .ghost()
        .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
        .selected(active)
        .child(
            div()
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                .child(theme::small_icon(icon))
                .child(
                    div()
                        .workbench_body()
                        .child(fileatlas_ui_bridge::i18n::tr(&label)),
                )
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
        .ml(px(fileatlas_ui_bridge::theme::space_compact()))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            theme::small_icon(IconName::X)
                .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::AccentText),
        )
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            clear(event, window, cx);
        })
        .into_any_element()
}

pub(super) fn popover_title(title: &str) -> Div {
    div()
        .px(px(fileatlas_ui_bridge::theme::space_standard()))
        .pb(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_assistant()
        .child(fileatlas_ui_bridge::i18n::tr(title))
}

pub(super) fn row_label(label: impl Into<String>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .workbench_body()
        .child(label.into())
}

pub(super) fn menu_row(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .h_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .px(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
        .cursor_pointer()
        .workbench_hover_surface()
}

pub(super) fn empty_hint(message: &str) -> Div {
    div()
        .px(px(fileatlas_ui_bridge::theme::space_standard()))
        .py(px(fileatlas_ui_bridge::theme::space_standard()))
        .workbench_assistant()
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
        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
        .pb(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(include)
        .child(exclude)
}

fn segment(id: &'static str, label: &'static str, selected: bool) -> Button {
    fileatlas_ui_bridge::components::button(id)
        .ghost()
        .flex_1()
        .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Medium)
        .selected(selected)
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
        .child(fileatlas_ui_bridge::components::range_slider_visual(
            RATING_SLIDER_PAD,
            RATING_SLIDER_PAD + track,
            low,
            high,
            true,
            true,
        ))
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

/// Both native triggers and plugin MCP control the same Popover open state.
pub(super) fn controlled_popover(
    view: &FileManagerView,
    weak: WeakEntity<FileManagerView>,
    id: &'static str,
    name: &'static str,
) -> Popover {
    fileatlas_ui_bridge::components::popover(id)
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
