use super::*;
use std::sync::Arc;

type Snapshot = Arc<dyn Fn() -> Result<SettingsPage, String> + Send + Sync>;
type Apply = Arc<dyn Fn(&str, SettingValue) -> Result<(), String> + Send + Sync>;

/// Build a settings view from declarations alone; register this contribution
/// through `register_views_for_plugin`. Snapshots MUST be memory-only. Complex
/// asynchronous editors can use the same schema in their own owned view factory.
pub fn contribution(
    id: impl Into<String>,
    title: impl Into<String>,
    description: impl Into<String>,
    snapshot: impl Fn() -> Result<SettingsPage, String> + Send + Sync + 'static,
    apply: impl Fn(&str, SettingValue) -> Result<(), String> + Send + Sync + 'static,
) -> crate::UiViewContribution {
    let snapshot: Snapshot = Arc::new(snapshot);
    let apply: Apply = Arc::new(apply);
    crate::UiViewContribution::new(id, "settings.pages", 0, move |_, app| {
        app.new(|_| ConfiguredPage {
            snapshot: snapshot.clone(),
            apply: apply.clone(),
            error: None,
        })
        .into()
    })
    .with_settings_page(title, description)
}

struct ConfiguredPage {
    snapshot: Snapshot,
    apply: Apply,
    error: Option<String>,
}
impl Render for ConfiguredPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let mut body = match (self.snapshot)() {
            Ok(page) => page.render_body(Rc::new(move |id, value, _, cx| {
                let _ = weak.update(cx, |view, cx| {
                    view.error = (view.apply)(id, value).err();
                    cx.notify();
                });
            })),
            Err(error) => div().text_color(rgb(theme::DANGER)).child(error),
        };
        if let Some(error) = &self.error {
            body = body.child(
                div()
                    .text_size(px(theme::FONT_SIZE_SMALL))
                    .text_color(rgb(theme::DANGER))
                    .child(error.clone()),
            );
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use super::{SettingsPage, contribution};
    #[test]
    fn settings_metadata_uses_existing_catalog_and_owner_cleanup() {
        let make = || {
            contribution(
                "example/settings",
                "Example",
                "",
                || Ok(SettingsPage::new("", "", vec![])),
                |_, _| Ok(()),
            )
        };
        let catalog = crate::UiViewCatalog::default();
        assert!(catalog.register("example.plugin", [make()]).is_ok());
        let Ok(first) = catalog.snapshot() else {
            panic!("catalog read failed")
        };
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].contribution.slot, "settings.pages");
        assert!(first[0].contribution.settings_page.is_some());
        assert!(catalog.register("example.plugin", [make()]).is_err());
        assert!(matches!(catalog.unregister_owner("example.plugin"), Ok(1)));
        assert!(matches!(catalog.snapshot(),Ok(rows) if rows.is_empty()));
        assert!(catalog.register("example.plugin", [make()]).is_ok());
        assert!(
            matches!(catalog.snapshot(), Ok(rows) if rows[0].generation != first[0].generation)
        );
    }
}
