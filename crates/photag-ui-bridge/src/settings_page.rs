//! Declarative settings presentation. Providers supply memory snapshots and own
//! validation/persistence; this module only renders controls and emits edits.
use crate::{components, i18n::tr, theme};
use gpui_kit::assets::IconName;
use gpui_kit::base::{Disableable as _, StyledExt as _};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::InputState;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{Sizable as _, Size};
use gpui_kit::*;
use std::rc::Rc;

/// Typed edits have no implicit file, database or global-settings behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingValue {
    Bool(bool),
    Choice(String),
    Invoke,
}
pub type SettingHandler = Rc<dyn Fn(&str, SettingValue, &mut Window, &mut App)>;

pub struct Choice {
    pub value: String,
    pub label: String,
    pub description: String,
    /// Content swatches may use the real user-selectable color.
    pub color: Option<u32>,
}
impl Choice {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: String::new(),
            color: None,
        }
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = value.into();
        self
    }
    pub fn swatch(mut self, value: u32) -> Self {
        self.color = Some(value);
        self
    }
}

pub enum SettingControl {
    Toggle(bool),
    InlineToggle {
        value: bool,
        label: String,
    },
    Select {
        value: String,
        options: Vec<Choice>,
    },
    Radios {
        value: String,
        options: Vec<Choice>,
    },
    Segments {
        value: String,
        options: Vec<Choice>,
    },
    Swatches {
        value: String,
        options: Vec<Choice>,
    },
    Action {
        label: String,
        primary: bool,
    },
    Text(Entity<InputState>),
    /// The owner opens its native picker; the renderer never performs IO.
    Path {
        value: String,
        browse_label: String,
    },
    ReadOnly(String),
    Status {
        value: String,
        enabled: bool,
    },
    /// Complex editors/shortcut capture retain their own state and semantics.
    Custom(AnyElement),
}

pub struct SettingRow {
    pub id: String,
    pub label: String,
    pub description: String,
    pub control: SettingControl,
    pub disabled: bool,
    pub error: Option<String>,
}
impl SettingRow {
    pub fn new(id: impl Into<String>, label: impl Into<String>, control: SettingControl) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: String::new(),
            control,
            disabled: false,
            error: None,
        }
    }
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = value.into();
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn error(mut self, value: Option<String>) -> Self {
        self.error = value;
        self
    }
}
pub struct SettingsSection {
    pub title: String,
    pub rows: Vec<SettingRow>,
}
impl SettingsSection {
    pub fn new(title: impl Into<String>, rows: Vec<SettingRow>) -> Self {
        Self {
            title: title.into(),
            rows,
        }
    }
}
pub struct SettingsPage {
    pub title: String,
    pub description: String,
    pub sections: Vec<SettingsSection>,
}
impl SettingsPage {
    /// Prevent ambiguous edits when a provider accidentally reuses a field ID.
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        for row in self.sections.iter().flat_map(|section| &section.rows) {
            if row.id.trim().is_empty() || !ids.insert(&row.id) {
                return Err(format!(
                    "Settings field ID is empty or duplicated: {}",
                    row.id
                ));
            }
            let options = match &row.control {
                SettingControl::Select { options, .. }
                | SettingControl::Radios { options, .. }
                | SettingControl::Segments { options, .. }
                | SettingControl::Swatches { options, .. } => Some(options),
                _ => None,
            };
            if let Some(options) = options {
                let mut values = std::collections::BTreeSet::new();
                if options.iter().any(|option| !values.insert(&option.value)) {
                    return Err(format!("Settings choice value is duplicated: {}", row.id));
                }
            }
        }
        Ok(())
    }
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        sections: Vec<SettingsSection>,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            sections,
        }
    }
    pub fn render(self, handler: SettingHandler) -> Div {
        div()
            .v_flex()
            .w_full()
            .min_w_0()
            .child(page_heading(&self.title, &self.description))
            .child(self.render_body(handler))
    }
    pub fn render_body(self, handler: SettingHandler) -> Div {
        if let Err(error) = self.validate() {
            return div().text_color(rgb(theme::DANGER)).child(error);
        }
        let mut page = div().v_flex().w_full().min_w_0().gap(px(theme::SPACE_2));
        for section in self.sections {
            if !section.title.is_empty() {
                page = page.child(section_heading(&section.title));
            }
            let divided = section.rows.len() > 1
                && section
                    .rows
                    .iter()
                    .all(|row| matches!(row.control, SettingControl::Toggle(_)));
            let count = section.rows.len();
            for (index, row) in section.rows.into_iter().enumerate() {
                let mut rendered = render_row(row, handler.clone());
                if divided && index + 1 < count {
                    rendered = rendered.border_b_1().border_color(rgb(theme::SEPARATOR));
                }
                page = page.child(rendered);
            }
        }
        page
    }
}

#[cfg(test)]
mod tests {
    use super::{Choice, SettingControl, SettingRow, SettingsPage, SettingsSection};
    #[test]
    fn rejects_duplicate_ids_across_sections() {
        let page = SettingsPage::new(
            "",
            "",
            vec![
                SettingsSection::new(
                    "A",
                    vec![SettingRow::new("same", "A", SettingControl::Toggle(true))],
                ),
                SettingsSection::new(
                    "B",
                    vec![SettingRow::new("same", "B", SettingControl::Toggle(false))],
                ),
            ],
        );
        assert!(page.validate().is_err());
    }
    #[test]
    fn rejects_ambiguous_choice_values() {
        let page = SettingsPage::new(
            "",
            "",
            vec![SettingsSection::new(
                "A",
                vec![SettingRow::new(
                    "mode",
                    "Mode",
                    SettingControl::Select {
                        value: "a".into(),
                        options: vec![Choice::new("a", "One"), Choice::new("a", "Two")],
                    },
                )],
            )],
        );
        assert!(page.validate().is_err());
    }
    #[test]
    fn accepts_typed_sections_without_a_new_store() {
        let page = SettingsPage::new(
            "Example",
            "",
            vec![SettingsSection::new(
                "General",
                vec![
                    SettingRow::new("enabled", "Enabled", SettingControl::Toggle(true)),
                    SettingRow::new(
                        "path",
                        "Folder",
                        SettingControl::Path {
                            value: "C:/Example".into(),
                            browse_label: "Choose".into(),
                        },
                    ),
                ],
            )],
        );
        assert!(page.validate().is_ok());
    }
}

pub fn page_heading(title: &str, description: &str) -> Div {
    div()
        .v_flex()
        .gap(px(theme::SPACE_2))
        .pb(px(24.))
        .child(
            div()
                .text_size(px(theme::SETTINGS_TITLE_SIZE))
                .font_semibold()
                .child(tr(title)),
        )
        .child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(tr(description)),
        )
}
pub fn section_heading(title: &str) -> Div {
    components::section_heading(tr(title))
        .text_size(px(theme::SETTINGS_SECTION_SIZE))
        .w_full()
        .pt(px(theme::SPACE_3))
        .pb(px(theme::SPACE_2))
        .border_t_1()
        .border_color(rgb(theme::SEPARATOR))
}
pub fn row() -> Div {
    div()
        .h_flex()
        .w_full()
        .min_w_0()
        .min_h(px(48.))
        .items_center()
        .justify_between()
        .gap(px(theme::SPACE_4))
}
pub fn label(title: &str, note: &str) -> Div {
    let mut label = div()
        .v_flex()
        .flex_1()
        .min_w_0()
        .gap(px(theme::SPACE_1))
        .child(div().text_size(px(theme::FONT_SIZE_BODY)).child(tr(title)));
    if !note.is_empty() {
        label = label.child(
            div()
                .text_size(px(theme::FONT_SIZE_SMALL))
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(tr(note)),
        );
    }
    label
}

mod render;
pub use render::render_row;
mod provider;
pub use provider::contribution;
