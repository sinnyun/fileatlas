use fileatlas_ui_bridge::components::{WorkbenchStyle as _, WorkbenchTypography as _};
use std::sync::Arc;
use std::time::Duration;

use fileatlas_ui_bridge::UiViewContribution;
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use crate::DevTestService;

pub(super) fn dev_test_panel(service: Arc<DevTestService>) -> UiViewContribution {
    UiViewContribution::new(
        "dev-test/panel-view",
        "workspace.dev-test",
        0,
        move |_, app| {
            app.new(|_| DevTestPanel {
                service: Arc::clone(&service),
            })
            .into()
        },
    )
}

struct DevTestPanel {
    service: Arc<DevTestService>,
}

pub(super) fn status_panel(service: Arc<DevTestService>, app: &mut App) -> AnyView {
    app.new(|cx| {
        let mut gate = fileatlas_ui_bridge::refresh::ChangeGate::default();
        fileatlas_ui_bridge::refresh::poll(
            cx,
            Duration::from_millis(200),
            move |view: &mut StatusPanel, _| gate.changed(view.0.status()).into(),
        );
        StatusPanel(service)
    })
    .into()
}

struct StatusPanel(Arc<DevTestService>);
impl Render for StatusPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
            .child("开发测试：真实扫描及规模任务状态")
            .child(if self.0.is_running() {
                "运行中"
            } else {
                "空闲"
            })
            .child(self.0.status())
    }
}

impl DevTestPanel {
    fn watch_status(&self, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.service);
        let mut gate = fileatlas_ui_bridge::refresh::ChangeGate::default();
        cx.notify();
        fileatlas_ui_bridge::refresh::poll(
            cx,
            Duration::from_millis(180),
            move |_: &mut Self, _| {
                let running = service.is_running();
                fileatlas_ui_bridge::refresh::PollResult::until(
                    gate.changed((running, service.status())),
                    running,
                )
            },
        );
    }
}

impl Render for DevTestPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .w_full()
            .gap(px(fileatlas_ui_bridge::theme::space_standard()))
            .p(px(fileatlas_ui_bridge::theme::space_expanded()))
            .workbench_separator(fileatlas_ui_bridge::components::DividerEdge::Top)
            .child(fileatlas_ui_bridge::components::heading_block(
                "开发测试：本地文件与规模数据",
                "样例扫描会读 test-data/fileatlas-fixtures 中的真实文件，并使用隔离的开发数据库。",
            ))
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap(px(fileatlas_ui_bridge::theme::space_standard()))
                    .child(
                        fileatlas_ui_bridge::components::button("dev-test-scan-fixtures")
                            .label(fileatlas_ui_bridge::i18n::tr(&("扫描样例目录").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.scan_fixture();
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("dev-test-seed-1k")
                            .label(fileatlas_ui_bridge::i18n::tr(&("注入 1 千").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.inject_virtual(1_000);
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("dev-test-seed-10k")
                            .label(fileatlas_ui_bridge::i18n::tr(&("注入 1 万").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.inject_virtual(10_000);
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("dev-test-seed-100k")
                            .label(fileatlas_ui_bridge::i18n::tr(&("注入 10 万").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.inject_virtual(100_000);
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        fileatlas_ui_bridge::components::button("dev-test-clear-virtual")
                            .label(fileatlas_ui_bridge::i18n::tr(&("清理虚拟记录").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.clear_virtual();
                                this.watch_status(cx);
                            })),
                    ),
            )
            .child(div().workbench_assistant().child(self.service.status()))
    }
}
