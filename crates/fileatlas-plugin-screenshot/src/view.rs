use crate::{PANEL, ScreenshotService};
use fileatlas_ui_bridge::components::WorkbenchForeground as _;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::{
    UiViewContribution, components, components::WorkbenchStyle as _, panel_catalog::PanelCatalog,
    theme,
};
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::*;
use open;
use std::sync::Arc;

pub(super) fn sidebar_button(
    panels: Arc<fileatlas_ui_bridge::panel_catalog::PanelCatalog>,
) -> UiViewContribution {
    UiViewContribution::new(
        "screenshot-tool/sidebar-button-view",
        "navigation.launcher-shortcuts",
        7,
        move |_, app| {
            let panels = Arc::clone(&panels);
            app.new(|_| ScreenshotButton { panels }).into()
        },
    )
}

struct ScreenshotButton {
    panels: Arc<fileatlas_ui_bridge::panel_catalog::PanelCatalog>,
}

impl Render for ScreenshotButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        components::rail_shortcut(
            "screenshot-tool-open-sidebar",
            "截图",
            theme::icon(IconName::Camera),
            false,
        )
        .on_click(cx.listener(|this, _, _, _| {
            if let Err(error) = this.panels.open(PANEL) {
                tracing::error!(%error, "could not open screenshot tool manager");
            }
        }))
        .into_any_element()
    }
}

pub(super) fn panel(
    service: Arc<ScreenshotService>,
    panels: Arc<PanelCatalog>,
    _window: &mut Window,
    app: &mut App,
) -> AnyView {
    app.new(|_| ScreenshotManagerPanel {
        service,
        panels,
        error: None,
    })
    .into()
}

struct ScreenshotManagerPanel {
    service: Arc<ScreenshotService>,
    panels: Arc<PanelCatalog>,
    error: Option<String>,
}

impl ScreenshotManagerPanel {
    fn choose_executable(&mut self, cx: &mut Context<Self>) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("选择 Shotoshot 程序")
            .add_filter("Shotoshot executable", &["exe"])
            .pick_file()
        else {
            return;
        };
        self.error = self.service.set_executable(path).err();
        cx.notify();
    }

    fn choose_directory(&mut self, cx: &mut Context<Self>) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("选择截图保存目录")
            .pick_folder()
        else {
            return;
        };
        self.error = self.service.set_save_directory(path).err();
        cx.notify();
    }

    fn launch(&mut self, cx: &mut Context<Self>) {
        self.error = self.service.launch_capture().err();
        cx.notify();
    }
}

impl Render for ScreenshotManagerPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let configuration = self.service.configuration().ok();
        let executable = self.service.installed_executable();
        let directory = configuration
            .as_ref()
            .and_then(|settings| settings.save_directory.as_ref());
        let available = executable.is_some();
        let startup_error = self.service.startup_error();
        let capture_action = self.service.mcp_state()["last_capture_action"]
            .as_str()
            .map(str::to_owned);

        let executable_label = executable.as_ref().map_or_else(
            || "未检测到 Shotoshot，请先安装并选择程序".to_owned(),
            |path| path.display().to_string(),
        );
        let directory_label = directory.map_or_else(
            || "%LOCALAPPDATA%\\FileAtlas\\Screenshots".to_owned(),
            |path| path.display().to_string(),
        );

        let mut executable_details = div()
            .v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(fileatlas_ui_bridge::theme::space_compact()))
            .child(components::section_heading("Shotoshot 程序"))
            .child(
                div()
                    .workbench_assistant()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(executable_label),
            );
        if !available {
            executable_details = executable_details.child(
                fileatlas_ui_bridge::components::button("screenshot-tool-install-shotoshot")
                    .ghost()
                    .label("获取 Shotoshot")
                    .on_click(|_, _, _| {
                        let _ = open::that_detached("https://shotoshot.com/");
                    }),
            );
        }

        let executable_card =
            div()
                .id("screenshot-tool-executable-card")
                .workbench_card(false)
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(theme::icon(IconName::AppWindow).workbench_foreground(
                    fileatlas_ui_bridge::components::ForegroundRole::AccentText,
                ))
                .child(executable_details)
                .child(
                    fileatlas_ui_bridge::components::button("screenshot-tool-choose-executable")
                        .outline()
                        .label("选择程序")
                        .on_click(cx.listener(|this, _, _, cx| this.choose_executable(cx))),
                );

        let directory_card =
            div()
                .id("screenshot-tool-directory-card")
                .workbench_card(false)
                .h_flex()
                .items_center()
                .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                .p(px(fileatlas_ui_bridge::theme::space_expanded()))
                .child(theme::icon(IconName::Folder).workbench_foreground(
                    fileatlas_ui_bridge::components::ForegroundRole::AccentText,
                ))
                .child(
                    div()
                        .v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(fileatlas_ui_bridge::theme::space_compact()))
                        .child(components::section_heading("截图保存目录"))
                        .child(
                            div()
                                .workbench_assistant()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(directory_label),
                        )
                        .child(
                            div()
                                .workbench_assistant()
                                .child("已同步到 Shotoshot 的保存位置设置"),
                        ),
                )
                .child(
                    fileatlas_ui_bridge::components::button("screenshot-tool-choose-directory")
                        .outline()
                        .label("更改目录")
                        .on_click(cx.listener(|this, _, _, cx| this.choose_directory(cx))),
                );

        let mut body = div()
            .v_flex()
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child(
                div()
                    .workbench_assistant()

                    .child("FileAtlas 启动时检测一次 Shotoshot。截图时由 FileAtlas 转发捕获请求，截图、选区、标注和编辑均由 Shotoshot 完成；截图保存到 FileAtlas 设置的目录。快捷键可在设置中修改。"),
            )
            .child(executable_card)
            .child(directory_card);

        if let Some(error) = self.error.as_ref().or(startup_error.as_ref()) {
            body = body.child(
                div()
                    .id("screenshot-tool-error")
                    .workbench_assistant()
                    .child(error.clone()),
            );
        }

        if let Some(action) = capture_action {
            let status = match action.as_str() {
                "delivered_to_running_shotoshot" => {
                    "已向正在运行的 Shotoshot 发送截图请求。若未出现编辑器，请查看下方错误信息和调试面板。"
                }
                "started_shotoshot_process" => "已启动 Shotoshot 截图进程。",
                _ => "",
            };
            if !status.is_empty() {
                body = body.child(
                    div()
                        .id("screenshot-tool-capture-status")
                        .workbench_assistant()
                        .child(status),
                );
            }
        }

        if let Some(error) = self.service.mcp_state()["last_capture_error"].as_str() {
            body = body.child(
                div()
                    .id("screenshot-tool-last-capture-error")
                    .workbench_assistant()
                    .child(format!("最近一次启动失败：{error}")),
            );
        }

        body.child(
            div()
                .h_flex()
                .justify_end()
                .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                .pt(px(fileatlas_ui_bridge::theme::space_standard()))
                .child(
                    fileatlas_ui_bridge::components::button("screenshot-tool-close")
                        .label("关闭")
                        .on_click(cx.listener(|this, _, _, cx| {
                            let _ = this.panels.close_definition(PANEL);
                            cx.notify();
                        })),
                )
                .child(
                    fileatlas_ui_bridge::components::button("screenshot-tool-launch")
                        .primary()
                        .label("启动截图　Ctrl+Shift+S")
                        .disabled(!available)
                        .on_click(cx.listener(|this, _, _, cx| this.launch(cx))),
                ),
        )
    }
}
