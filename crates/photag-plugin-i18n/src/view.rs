use crate::{I18nService, Language};
use gpui_kit::*;
use photag_ui_bridge::{
    UiViewContribution,
    settings_page::{
        Choice, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
    },
};
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
        let weak = cx.entity().downgrade();
        SettingsPage::new(
            "",
            "",
            vec![SettingsSection::new(
                "",
                vec![
                    SettingRow::new(
                        "i18n-language",
                        "Display language",
                        SettingControl::Select {
                            value: service.language().code().into(),
                            options: vec![
                                Choice::new("en", "English"),
                                Choice::new("zh-CN", "简体中文"),
                            ],
                        },
                    )
                    .description("Changes take effect immediately.")
                    .error(self.error.clone()),
                ],
            )],
        )
        .render_body(std::rc::Rc::new(move |_, value, _, cx| {
            if let SettingValue::Choice(value) = value {
                let _ = weak.update(cx, |view, cx| {
                    view.error = Language::parse(&value)
                        .and_then(|language| {
                            view.service
                                .upgrade()
                                .map_or(Ok(()), |service| service.set_language(language))
                        })
                        .err()
                        .map(|error| error.to_string());
                    cx.refresh_windows();
                });
            }
        }))
    }
}
