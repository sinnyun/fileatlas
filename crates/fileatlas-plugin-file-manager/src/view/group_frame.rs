//! One group owns its title, media area and file-drop destination. Large groups
//! paint adjoining segments so the FileManager retains row-level virtualization.
use super::*;
use gpui_kit::prelude::FluentBuilder as _;

pub(super) fn render(
    row_index: usize,
    row: &FileManagerVisualRow,
    header: AnyElement,
    files: AnyElement,
    view: &FileManagerView,
    cx: &mut Context<FileManagerView>,
) -> AnyElement {
    let first = row.month_label.is_some();
    let grouped = row.group_label.is_some();
    let hovered = row.group_label.is_some()
        && view
            .surfaces
            .value(&hover_key(view.column_index))
            .as_deref()
            == row.group_label.as_deref();
    let mut frame = div()
        .id(format!("file_manager-group-frame-{row_index}"))
        .relative()
        .v_flex()
        .w_full()
        .h_full()
        .when(hovered, |frame| frame.bg(rgb(theme::selection_color())))
        .child(header)
        .child(files);
    if grouped {
        frame = frame.child(
            div()
                .absolute()
                .inset_0()
                .border_l_1()
                .border_r_1()
                .when(first, |edge| {
                    edge.border_t_1().rounded_t(px(theme::ROUND_SM))
                })
                .when(row.group_end, |edge| {
                    edge.border_b_1().rounded_b(px(theme::ROUND_SM))
                })
                .border_color(rgb(if hovered {
                    theme::accent_hover()
                } else {
                    theme::separator()
                })),
        );
    }
    if let Some(destination) = row
        .group_label
        .clone()
        .filter(|_| view.grouping_mode == fileatlas_plugin_grouping::GroupingMode::Subfolder)
    {
        let surfaces = Arc::clone(&view.surfaces);
        let column = view.column_index;
        let measured_path = destination.clone();
        let native_destination = destination.clone();
        frame = frame
            .child(
                canvas(
                    move |bounds, _, _| {
                        let key = format!("file_manager.column.{column}.group-frames");
                        let mut regions = surfaces
                            .value(&key)
                            .and_then(|value| {
                                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
                                    &value,
                                )
                                .ok()
                            })
                            .unwrap_or_default();
                        regions.insert(
                            row_index.to_string(),
                            serde_json::json!({
                                "path": measured_path, "first": first,
                                "bounds": [f32::from(bounds.origin.x), f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width), f32::from(bounds.size.height)]
                            }),
                        );
                        let _ = surfaces
                            .set_value(&key, &serde_json::Value::Object(regions).to_string());
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .on_drop::<fileatlas_contracts::FileManagerFileDragPayload>(cx.listener(
                move |this, payload: &fileatlas_contracts::FileManagerFileDragPayload, _, cx| {
                    let _ = this.surfaces.set_value(&hover_key(this.column_index), "");
                    if payload.allows_drop() {
                        this.move_ids_to_directory(
                            payload.ids.clone(),
                            native_destination.clone(),
                            cx,
                        );
                    }
                },
            ));
        let staging_destination = destination.clone();
        frame = frame
            .drag_over::<fileatlas_contracts::FileStagingDragPayload>(|style, _, _, _| {
                style.bg(rgb(theme::selection_color()))
            })
            .on_drop::<fileatlas_contracts::FileStagingDragPayload>(cx.listener(
                move |this, payload: &fileatlas_contracts::FileStagingDragPayload, _, cx| {
                    let _ = this.surfaces.set_value(&hover_key(this.column_index), "");
                    this.move_staged_paths_to_directory(
                        payload.paths.clone(),
                        staging_destination.clone(),
                        cx,
                    );
                },
            ));
    }
    frame.into_any_element()
}

pub(super) fn hover_key(column: usize) -> String {
    format!("file_manager.column.{column}.group-drop-hover")
}

/// The visible segments project one group-wide drag state. Notify only when
/// the target directory changes, including leaving the FileManager viewport.
pub(super) fn track_drag(
    view: &FileManagerView,
    event: &::gpui::DragMoveEvent<fileatlas_contracts::FileManagerFileDragPayload>,
    cx: &mut Context<FileManagerView>,
) {
    let destination = (event.drag(cx).allows_drop()
        && event.bounds.contains(&event.event.position))
    .then(|| destination_at(view, event.event.position))
    .flatten();
    set_hover_destination(view, destination, cx);
}

pub(super) fn track_staging_drag(
    view: &FileManagerView,
    event: &::gpui::DragMoveEvent<fileatlas_contracts::FileStagingDragPayload>,
    cx: &mut Context<FileManagerView>,
) {
    let destination = event
        .bounds
        .contains(&event.event.position)
        .then(|| destination_at(view, event.event.position))
        .flatten();
    set_hover_destination(view, destination, cx);
}

pub(super) fn destination_at(view: &FileManagerView, position: Point<Pixels>) -> Option<String> {
    view.surfaces
        .value(&format!(
            "file_manager.column.{}.group-frames",
            view.column_index
        ))
        .and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok())
        .and_then(|regions| {
            regions.as_object().and_then(|regions| {
                regions.values().find_map(|region| {
                    let bounds = region["bounds"].as_array()?;
                    let x = bounds.first()?.as_f64()? as f32;
                    let y = bounds.get(1)?.as_f64()? as f32;
                    let w = bounds.get(2)?.as_f64()? as f32;
                    let h = bounds.get(3)?.as_f64()? as f32;
                    Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
                        .contains(&position)
                        .then(|| region["path"].as_str().map(str::to_owned))
                        .flatten()
                })
            })
        })
}

fn set_hover_destination(
    view: &FileManagerView,
    destination: Option<String>,
    cx: &mut Context<FileManagerView>,
) {
    let key = hover_key(view.column_index);
    let destination = destination.unwrap_or_default();
    if view.surfaces.value(&key).unwrap_or_default() != destination {
        let _ = view.surfaces.set_value(&key, &destination);
        cx.notify();
    }
}
