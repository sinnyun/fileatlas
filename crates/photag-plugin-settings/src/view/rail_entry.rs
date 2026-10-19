use super::*;
use gpui_kit::assets::IconName;

struct SettingsRailEntry {
    service: Arc<SettingsService>,
    surfaces: Arc<UiSurfaceState>,
}

impl Render for SettingsRailEntry {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_light = self
            .service
            .snapshot()
            .is_ok_and(|s| s.color_mode == "light");
        let theme_service = Arc::clone(&self.service);
        let mode = Button::new("settings-rail-theme")
            .ghost()
            .compact()
            .icon(theme::icon(if is_light {
                IconName::Moon
            } else {
                IconName::Sun
            }))
            .tooltip(photag_ui_bridge::i18n::tr(if is_light {
                "Switch to dark mode"
            } else {
                "Switch to light mode"
            }))
            .accessibility_label(photag_ui_bridge::i18n::tr("Toggle color mode"))
            .on_click(cx.listener(move |_, _, _, cx| {
                let next = if theme_service
                    .snapshot()
                    .is_ok_and(|s| s.color_mode == "light")
                {
                    "dark"
                } else {
                    "light"
                };
                match theme_service.update(|settings| settings.color_mode = next.into()) {
                    Ok(_) => {
                        photag_ui_bridge::theme::set_color_mode(if next == "light" {
                            theme::ColorMode::Light
                        } else {
                            theme::ColorMode::Dark
                        });
                        theme::apply_accent_components(cx);
                        cx.refresh_windows();
                        cx.notify();
                    }
                    Err(error) => tracing::error!(%error, "could not persist color mode"),
                }
            }));
        let surfaces = Arc::clone(&self.surfaces);
        let settings = Button::new("settings-rail-open")
            .ghost()
            .compact()
            .icon(theme::icon(IconName::Settings))
            .tooltip(photag_ui_bridge::i18n::tr("Settings"))
            .accessibility_label(photag_ui_bridge::i18n::tr("Settings"))
            .on_click(move |_, _, _| {
                if let Err(error) = surfaces.toggle("settings") {
                    tracing::error!(%error, "settings toggle failed");
                }
            });
        div()
            .v_flex()
            .w(px(48.))
            .h(px(96.))
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .gap(px(2.))
            .bg(rgb(theme::surface_rail()))
            .child(mode)
            .child(settings)
    }
}

pub(super) fn contribution(
    service: Arc<SettingsService>,
    surfaces: Arc<UiSurfaceState>,
) -> UiViewContribution {
    UiViewContribution::new(
        "settings/rail-entry",
        "workspace.sidebar-footer",
        0,
        move |_, app| {
            app.new(|_| SettingsRailEntry {
                service: Arc::clone(&service),
                surfaces: Arc::clone(&surfaces),
            })
            .into()
        },
    )
}
