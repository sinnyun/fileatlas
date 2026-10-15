use super::*;

#[derive(serde::Serialize)]
pub(super) struct TagChoice {
    pub id: i64,
    pub name: String,
    pub label: String,
}

impl SettingsView {
    pub(super) fn poll_tag_choices(&mut self) -> bool {
        if !self.surfaces.is_open("settings") {
            self.tag_choices = Default::default();
            return false;
        }
        if !self
            .surfaces
            .value("settings.tab")
            .is_some_and(|tab| matches!(tab.as_str(), "shortcuts" | "ai"))
        {
            return false;
        }
        let navigation = self.navigation.as_ref().and_then(Weak::upgrade);
        let key = (
            navigation.is_some(),
            navigation.as_ref().map_or(0, |n| n.tag_revision()),
        );
        let changed = self.tag_choices.update(key, move || {
            let navigation = navigation
                .ok_or("Tag choices are unavailable while the navigation plugin is stopped.")?;
            let snapshot = navigation.snapshot().map_err(|error| error.to_string())?;
            let mut pending: Vec<_> = snapshot.tags.into_iter().rev().collect();
            let mut choices = Vec::new();
            while let Some(tag) = pending.pop() {
                choices.push(TagChoice {
                    id: tag.id,
                    name: tag.name,
                    label: tag.full_path.replace('>', " > "),
                });
                pending.extend(tag.children.into_iter().rev());
            }
            Ok(choices)
        });
        if changed {
            let value = match self.tag_choices.value() {
                Some(Ok(choices)) => {
                    serde_json::json!({"state":"ready", "revision":key.1, "choices":choices})
                }
                Some(Err(error)) => serde_json::json!({"state":"failed", "error":error}),
                None => serde_json::json!({"state":"loading"}),
            };
            let _ = self
                .surfaces
                .set_value("settings.tag-choices", &value.to_string());
        }
        changed
    }
}
