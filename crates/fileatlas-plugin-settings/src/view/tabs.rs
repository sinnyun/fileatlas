use super::*;
use fileatlas_ui_bridge::components::WorkbenchTypography as _;
use fileatlas_ui_bridge::settings_page::{
    Choice, SettingControl, SettingRow, SettingValue, SettingsPage, SettingsSection,
};
impl SettingsView {
    pub(super) fn render_tab(
        &self,
        s: &Settings,
        width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.tab {
            Tab::Appearance => self.render_appearance(s, cx).into_any_element(),
            Tab::Shortcuts => self.render_shortcuts(s, width, cx).into_any_element(),
            Tab::Tags => self.render_tags(s, cx).into_any_element(),
            Tab::Ai => div()
                .v_flex()
                .gap(px(theme::space_expanded()))
                .child(self.section("Automatic analysis"))
                .child(self.toggle_button(
                    "ai",
                    "Automatically analyze opened photos",
                    s.ai.auto_analyze,
                    cx,
                ))
                .child(
                    div()
                        .h_flex()
                        .items_center()
                        .justify_between()
                        .gap(px(fileatlas_ui_bridge::theme::space_expanded()))
                        .child(self.section("Mandatory tag groups").flex_1())
                        .child(self.add_group_button(cx)),
                )
                .child(
                    div()
                        .workbench_assistant()
                        .child(fileatlas_ui_bridge::i18n::tr(
                            "Each group defines required tags for automatic analysis.",
                        )),
                )
                .child(self.ai_groups(s, cx))
                .into_any_element(),
            Tab::Updates => self.render_updates_page(s, cx).into_any_element(),
            Tab::Software => self.render_software_page(cx, false).into_any_element(),
            Tab::Plugins => self.render_plugin_cards(s, cx).into_any_element(),
            Tab::PluginSettings => self.render_plugin_settings(s, cx).into_any_element(),
        }
    }
    fn render_tags(&self, s: &Settings, cx: &mut Context<Self>) -> Div {
        let weak = cx.entity().downgrade();
        let scopes = [
            ("files", "Filenames", s.search_scopes.filenames),
            ("paths", "Paths", s.search_scopes.paths),
            ("tags", "Tags", s.search_scopes.tags),
            ("time", "Time range", s.search_scopes.time),
        ]
        .into_iter()
        .map(|(key, label, value)| {
            SettingRow::new(
                format!("settings-search.{key}"),
                label,
                SettingControl::Toggle(value),
            )
        })
        .collect();
        SettingsPage::new("","",vec![
            SettingsSection::new("Search match",vec![SettingRow::new("settings-tag-match","Tag name matching",SettingControl::Select {
                value:if s.tag_search_match==TagSearchMatch::Contains {"contains"} else {"starts"}.into(),
                options:vec![Choice::new("contains",TagSearchMatch::Contains.label()),Choice::new("starts",TagSearchMatch::StartsWith.label())],
            }).description("Choose whether a query can match anywhere or only at the start of a tag name.")]),
            SettingsSection::new("Search scope",scopes),
            SettingsSection::new("",vec![SettingRow::new("settings-persist","Keep search query when switching tabs",SettingControl::Toggle(s.tag_search_persist))]),
            SettingsSection::new("Source tag children after merge",vec![SettingRow::new("settings-tag-merge","",SettingControl::Radios {
                value:if s.tag_merge_children==TagMergeChildren::Reparent {"reparent"} else {"delete"}.into(),
                options:vec![Choice::new("reparent",TagMergeChildren::Reparent.label()),Choice::new("delete",TagMergeChildren::Delete.label())],
            })]),
        ]).render_body(std::rc::Rc::new(move |id,value,_,cx| {
            let _=weak.update(cx,|view,cx| view.edit(|settings| match (id,&value) {
                ("settings-tag-match",SettingValue::Choice(v))=>settings.tag_search_match=if v=="contains" {TagSearchMatch::Contains} else {TagSearchMatch::StartsWith},
                ("settings-tag-merge",SettingValue::Choice(v))=>settings.tag_merge_children=if v=="reparent" {TagMergeChildren::Reparent} else {TagMergeChildren::Delete},
                ("settings-search.files",SettingValue::Bool(v))=>settings.search_scopes.filenames=*v,
                ("settings-search.paths",SettingValue::Bool(v))=>settings.search_scopes.paths=*v,
                ("settings-search.tags",SettingValue::Bool(v))=>settings.search_scopes.tags=*v,
                ("settings-search.time",SettingValue::Bool(v))=>settings.search_scopes.time=*v,
                ("settings-persist",SettingValue::Bool(v))=>settings.tag_search_persist=*v,
                _=>{}
            },cx));
        }))
    }
}
