//! Settings content geometry; colors and primitive styling stay in UI Bridge.
use super::*;

pub(super) use fileatlas_ui_bridge::settings_page::{label, row, section_heading as heading};

impl SettingsView {
    pub(super) fn page_header(&self) -> Div {
        let (title, note) = match self.tab {
            Tab::Appearance => (
                "Appearance",
                "Choose the accent color used across FileAtlas.",
            ),
            Tab::Shortcuts => (
                "Shortcuts",
                "管理程序和插件按键。系统级快捷键也可在其他程序中使用。",
            ),
            Tab::Tags => (
                "Tags and search",
                "Configure tag search and child-tag handling after a merge.",
            ),
            Tab::Ai => (
                "AI analysis",
                "Configure automatic analysis and required tag groups.",
            ),
            Tab::Updates => (
                "Software updates",
                "Check for releases and choose when to install them.",
            ),
            Tab::Software => (
                "External software",
                "Check the external software used by FileAtlas.",
            ),
            Tab::Plugins => (
                "Plugin management",
                "管理工作台功能插件，设置会在停用后保留。",
            ),
            Tab::PluginSettings => ("Plugin settings", "Configure this feature plugin."),
        };
        let plugin = (self.tab == Tab::PluginSettings)
            .then(|| {
                self.service
                    .plugin_runtime()
                    .snapshot_now()
                    .into_iter()
                    .find(|p| Some(p.id.as_str()) == self.selected_plugin_id.as_deref())
            })
            .flatten();
        fileatlas_ui_bridge::settings_page::page_heading(
            plugin
                .as_ref()
                .map(|p| p.title.as_str())
                .or_else(|| {
                    self.plugin_settings_views
                        .iter()
                        .find(|v| Some(v.0.as_str()) == self.selected_plugin_id.as_deref())
                        .map(|v| v.1.as_str())
                        .filter(|_| self.tab == Tab::PluginSettings)
                })
                .unwrap_or(title),
            plugin.as_ref().map_or(note, |p| p.description.as_str()),
        )
    }
}
