//! Group section band; user-requested raised background distinguishes it from media rows.
use super::*;
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchPaint as _;
use fileatlas_ui_bridge::components::{WorkbenchStyle as _, WorkbenchTypography as _};

pub(super) fn render(
    label: String,
    count: usize,
    expanded: f32,
    scope: Option<&str>,
    cx: &mut Context<FileManagerView>,
) -> Stateful<Div> {
    let display = scope
        .and_then(|scope| std::path::Path::new(&label).strip_prefix(scope).ok())
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                fileatlas_ui_bridge::i18n::tr("Current folder")
            } else {
                relative.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| fileatlas_ui_bridge::i18n::tr(&label));
    let header = theme::separate_region(
        div()
            .relative()
            .w_full()
            .h(px(theme::GROUP_HEADER_HEIGHT))
            .workbench_surface(fileatlas_ui_bridge::components::SurfaceRole::GroupHeader)
            .workbench_corner(fileatlas_ui_bridge::components::CornerRole::Small)
            .h_flex()
            .items_center()
            .pl(px(fileatlas_ui_bridge::theme::space_expanded()))
            .pr(px(fileatlas_ui_bridge::theme::space_expanded()))
            .pt(px(fileatlas_ui_bridge::theme::space_compact()))
            .gap(px(fileatlas_ui_bridge::theme::space_standard())),
        theme::RegionEdge::All,
    )
    .id(format!("file_manager-month-{label}"))
    .child(
        div()
            .min_w_0()
            .overflow_hidden()
            .text_ellipsis()
            .workbench_title()
            .child(display),
    )
    .child(
        div()
            .workbench_assistant()
            .pt(px(fileatlas_ui_bridge::theme::space_compact()))
            .workbench_foreground(fileatlas_ui_bridge::components::ForegroundRole::Muted)
            .child("·"),
    )
    .child(
        div()
            .workbench_assistant()
            .pt(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(fileatlas_ui_bridge::i18n::tr_args(
                "{count} items",
                &[("count", count.to_string())],
            )),
    )
    .child(
        div()
            .absolute()
            .left(px(8.))
            .top_0()
            .h_full()
            .flex()
            .items_center()
            .child(theme::small_icon(if expanded >= 0.5 {
                gpui_kit::assets::IconName::ChevronDown
            } else {
                gpui_kit::assets::IconName::ChevronRight
            })),
    )
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .on_click(cx.listener({
        let label = label.clone();
        move |this, _, _, cx| {
            cx.stop_propagation();
            this.toggle_month_collapsed(label.clone(), cx);
        }
    }))
    .cursor_pointer()
    .workbench_hover_group_header();
    fileatlas_ui_bridge::components::review_section(
        header,
        fileatlas_ui_bridge::geometry_audit::ComponentTokens {
            height: Some(theme::GROUP_HEADER_HEIGHT),
            gap: Some(fileatlas_ui_bridge::theme::space_standard()),
            padding_x: Some(fileatlas_ui_bridge::theme::space_expanded()),
            padding_y: Some(fileatlas_ui_bridge::theme::space_compact()),
            ..Default::default()
        },
    )
}
