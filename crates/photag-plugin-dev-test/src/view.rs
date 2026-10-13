use std::sync::Arc;
use std::time::Duration;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use photag_ui_bridge::{UiViewContribution, theme};

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

impl DevTestPanel {
    fn watch_status(&self, cx: &mut Context<Self>) {
        let service = Arc::clone(&self.service);
        cx.spawn(async move |this, cx| {
            loop {
                let running = this
                    .update(cx, |_, cx| {
                        cx.notify();
                        service.is_running()
                    })
                    .unwrap_or(false);
                if !running {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(180))
                    .await;
            }
        })
        .detach();
    }
}

impl Render for DevTestPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .border_t_1()
            .border_color(rgb(theme::SURFACE_FAINT))
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .child("开发测试：媒体库与规模数据"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child("样例扫描会读 test-data/photag-library 中的真实文件，并使用隔离的开发数据库。"),
            )
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("dev-test-scan-fixtures")
                            .label(photag_ui_bridge::i18n::tr(&("扫描样例目录").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.scan_fixture();
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        Button::new("dev-test-seed-1k")
                            .label(photag_ui_bridge::i18n::tr(&("注入 1 千").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.inject_virtual(1_000);
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        Button::new("dev-test-seed-10k")
                            .label(photag_ui_bridge::i18n::tr(&("注入 1 万").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.inject_virtual(10_000);
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        Button::new("dev-test-seed-100k")
                            .label(photag_ui_bridge::i18n::tr(&("注入 10 万").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.inject_virtual(100_000);
                                this.watch_status(cx);
                            })),
                    )
                    .child(
                        Button::new("dev-test-clear-virtual")
                            .label(photag_ui_bridge::i18n::tr(&("清理虚拟记录").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.service.clear_virtual();
                                this.watch_status(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_SECONDARY))
                    .child(self.service.status()),
            )
    }
}
