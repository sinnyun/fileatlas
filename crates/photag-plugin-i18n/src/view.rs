use crate::{I18nService, Language};
use gpui_kit::assets::IconName;
use gpui_kit::{base::StyledExt, component::button::Button, *};
use photag_ui_bridge::{UiViewContribution, i18n::tr, theme};
use std::sync::Weak;

pub(super) fn language(service: Weak<I18nService>) -> UiViewContribution {
    UiViewContribution::new(
        "i18n/language-selector",
        "settings.language",
        0,
        move |_, cx| {
            cx.new(|_| LanguageView {
                service: service.clone(),
                error: None,
            })
            .into()
        },
    )
}

struct LanguageView {
    service: Weak<I18nService>,
    error: Option<String>,
}
impl Render for LanguageView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(service) = self.service.upgrade() else {
            return div();
        };
        let selected = service.language();
        let mut row = div()
            .v_flex()
            .gap(px(6.))
            .child(div().font_semibold().child(tr("Language")))
            .child(
                div().h_flex().gap(px(8.)).children(
                    [
                        (Language::English, "English"),
                        (Language::Chinese, "简体中文"),
                    ]
                    .map(|(language, label)| {
                        Button::new(format!("i18n-{}", language.code()))
                            .icon(theme::small_icon(if selected == language {
                                IconName::CircleDot
                            } else {
                                IconName::Circle
                            }))
                            .label(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.error = this.service.upgrade().and_then(|service| {
                                    service
                                        .set_language(language)
                                        .err()
                                        .map(|error| error.to_string())
                                });
                                cx.refresh_windows();
                            }))
                    }),
                ),
            );
        if let Some(error) = &self.error {
            row = row.child(div().text_color(rgb(theme::DANGER)).child(error.clone()));
        }
        row
    }
}
