use super::*;

pub(crate) fn measure(
    id: &str,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
) -> impl IntoElement {
    let key = format!("workcrate.control.{id}");
    gpui::canvas(
        move |bounds, _, _| {
            let value = format!(
                "{},{},{},{}",
                f32::from(bounds.origin.x),
                f32::from(bounds.origin.y),
                f32::from(bounds.size.width),
                f32::from(bounds.size.height)
            );
            if surfaces.value(&key).as_deref() != Some(&value) {
                let _ = surfaces.set_value(&key, &value);
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .left_0()
    .top_0()
    .size_full()
}

pub(crate) fn control(
    id: &str,
    element: impl IntoElement,
    surfaces: &Arc<fileatlas_ui_bridge::UiSurfaceState>,
) -> gpui::Div {
    div()
        .relative()
        .flex_shrink_0()
        .child(element)
        .child(measure(id, Arc::clone(surfaces)))
}
