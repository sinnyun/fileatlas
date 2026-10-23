//! Source-painted 22-point sliders (filterbar.rs:600-771).
use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;

#[derive(Clone, Copy)]
pub(super) enum SliderDrag {
    Size,
    RatingMin,
    RatingMax,
}

fn slider(
    view: &FileManagerView,
    rating: bool,
    cx: &mut Context<FileManagerView>,
) -> impl IntoElement {
    let width = if rating { 150. } else { 116. };
    let pad = if rating { 9. } else { 8. };
    let track = width - pad * 2.;
    let (low, high) = if rating {
        (
            pad + view.filter.rating_min as f32 / 5. * track,
            pad + view.filter.rating_max as f32 / 5. * track,
        )
    } else {
        (pad, pad + (view.thumb_size - 80.) / 220. * track)
    };
    let bounds = Arc::new(std::sync::Mutex::new(Bounds::<Pixels>::default()));
    let layout_bounds = bounds.clone();
    let down_bounds = bounds.clone();
    let move_bounds = bounds;
    let active = !rating || view.filter.rating_min != 0 || view.filter.rating_max != 5;
    let root = div()
        .id(if rating {
            "file_manager-rating-slider"
        } else {
            "file_manager-thumb-slider"
        })
        .relative()
        .w(px(width))
        .h(px(22.))
        .flex_shrink_0()
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
            pad,
            width - pad,
            low,
            high,
            active,
            rating,
        ))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                let x = f32::from(event.position.x - down_bounds.lock().unwrap().origin.x);
                this.slider_drag = Some(if rating {
                    if (x - low).abs() < (x - high).abs() {
                        SliderDrag::RatingMin
                    } else {
                        SliderDrag::RatingMax
                    }
                } else {
                    SliderDrag::Size
                });
                this.change_slider(x, pad, track, cx);
            }),
        )
        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            if event.dragging() && this.slider_drag.is_some() {
                let x = f32::from(event.position.x - move_bounds.lock().unwrap().origin.x);
                this.change_slider(x, pad, track, cx);
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _, _, _| this.slider_drag = None),
        );
    root
}

pub(super) fn thumbnail(view: &FileManagerView, cx: &mut Context<FileManagerView>) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap(px(fileatlas_ui_bridge::theme::space_standard()))
        .child(
            div()
                .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Secondary)
                .child(theme::small_icon(gpui_kit::assets::IconName::ZoomIn)),
        )
        .child(slider(view, false, cx))
}

impl FileManagerView {
    pub(super) fn change_slider(&mut self, x: f32, pad: f32, track: f32, cx: &mut Context<Self>) {
        let fraction = ((x - pad) / track).clamp(0., 1.);
        match self.slider_drag {
            Some(SliderDrag::Size) => {
                self.thumb_size = 80. + fraction * 220.;
                self.row_layout_key = None;
                self.persist_filter_session();
                cx.notify();
            }
            Some(SliderDrag::RatingMin) => {
                self.set_rating_bound(true, (fraction * 5.).round() as i32, cx)
            }
            Some(SliderDrag::RatingMax) => {
                self.set_rating_bound(false, (fraction * 5.).round() as i32, cx)
            }
            None => {}
        }
    }
}
