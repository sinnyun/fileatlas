//! GPUI settings surface migrated from the pinned PhoTag settings dialog.
use crate::{
    AiTagGroup, ExcludeModifier, Settings, SettingsService, TagMergeChildren, TagSearchMatch,
};
use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_ui_bridge::UiViewContribution;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Appearance,
    Toolbar,
    Shortcuts,
    Tags,
    Ai,
    Updates,
}
pub(super) fn settings(service: Arc<SettingsService>) -> UiViewContribution {
    UiViewContribution::new(
        "settings/panel-view",
        "workspace.settings",
        0,
        move |_, app| {
            app.new(|_| SettingsView {
                service: Arc::clone(&service),
                tab: Tab::Appearance,
                error: None,
            })
            .into()
        },
    )
}
struct SettingsView {
    service: Arc<SettingsService>,
    tab: Tab,
    error: Option<String>,
}
impl SettingsView {
    fn edit(&mut self, f: impl Fn(&mut Settings), cx: &mut Context<Self>) {
        match self.service.update(f) {
            Ok(_) => self.error = None,
            Err(e) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn toggle(&mut self, key: &'static str, cx: &mut Context<Self>) {
        self.edit(
            |s| match key {
                "date" => s.show_date_dividers = !s.show_date_dividers,
                "persist" => s.tag_search_persist = !s.tag_search_persist,
                "filmstrip" => s.show_filmstrip = !s.show_filmstrip,
                "updates" => s.auto_check_updates = !s.auto_check_updates,
                "ai" => s.ai.auto_analyze = !s.ai.auto_analyze,
                "search.files" => s.search_scopes.filenames = !s.search_scopes.filenames,
                "search.paths" => s.search_scopes.paths = !s.search_scopes.paths,
                "search.tags" => s.search_scopes.tags = !s.search_scopes.tags,
                "pos.locked" => s.toolbar_pos.locked = !s.toolbar_pos.locked,
                "tb.zoom_mode" => s.toolbar.zoom_mode_button = !s.toolbar.zoom_mode_button,
                "tb.zoom_reset" => s.toolbar.zoom_reset_button = !s.toolbar.zoom_reset_button,
                "tb.zoom_presets" => s.toolbar.zoom_presets = !s.toolbar.zoom_presets,
                "tb.rotate" => s.toolbar.rotate_buttons = !s.toolbar.rotate_buttons,
                "tb.flip" => s.toolbar.flip_buttons = !s.toolbar.flip_buttons,
                "tb.tags" => s.toolbar.tags_button = !s.toolbar.tags_button,
                "tb.delete" => s.toolbar.delete_button = !s.toolbar.delete_button,
                "tb.copy" => s.toolbar.copy_button = !s.toolbar.copy_button,
                "tb.move" => s.toolbar.move_button = !s.toolbar.move_button,
                "tb.filmstrip" => s.toolbar.filmstrip_toggle = !s.toolbar.filmstrip_toggle,
                "tb.explorer" => s.toolbar.explorer_button = !s.toolbar.explorer_button,
                "tb.external" => s.toolbar.open_external_button = !s.toolbar.open_external_button,
                _ => {}
            },
            cx,
        );
    }
    fn toggle_button(
        &self,
        key: &'static str,
        label: &str,
        value: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(format!("settings-{key}"))
            .label(format!("{}  [{}]", label, if value { "On" } else { "Off" }))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle(key, cx)))
    }
    fn enum_button(
        &self,
        id: &'static str,
        label: String,
        action: impl Fn(&mut Settings) + Send + Sync + 'static,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(format!("settings-{id}"))
            .label(label)
            .on_click(cx.listener(move |this, _, _, cx| this.edit(action, cx)))
    }
    fn section(&self, label: &str) -> Div {
        div().pt_2().font_semibold().child(label)
    }
    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut row = div().h_flex().gap_1().flex_wrap();
        for (tab, name) in [
            (Tab::Appearance, "Appearance"),
            (Tab::Toolbar, "Toolbar"),
            (Tab::Shortcuts, "Shortcuts"),
            (Tab::Tags, "Tags"),
            (Tab::Ai, "AI"),
            (Tab::Updates, "Updates"),
        ] {
            let label = if self.tab == tab {
                format!("• {name}")
            } else {
                name.into()
            };
            row = row.child(
                Button::new(format!("settings-tab-{name}"))
                    .label(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab;
                        cx.notify()
                    })),
            );
        }
        row
    }
    fn render_tab(&self, s: &Settings, cx: &mut Context<Self>) -> impl IntoElement {
        let mut b = div().v_flex().gap_2().p_3();
        match self.tab {
            Tab::Appearance => {
                b = b.child(self.section("Accent color"));
                for name in [
                    "System",
                    "Slate Blue",
                    "Dusty Teal",
                    "Muted Copper",
                    "Dusty Rose",
                    "Sage Green",
                    "Dusty Mauve",
                    "Warm Taupe",
                ] {
                    let selected = s.accent_name == name;
                    let value = name.to_owned();
                    b = b.child(self.enum_button(
                        "accent",
                        format!("{} {name}", if selected { "●" } else { "○" }),
                        move |s| s.accent_name = value.clone(),
                        cx,
                    ));
                }
                b = b.child(self.toggle_button(
                    "date",
                    "Group date-sorted gallery by month",
                    s.show_date_dividers,
                    cx,
                ));
            }
            Tab::Toolbar => {
                b = b.child(self.section("Preview toolbar controls"));
                for (k, n, v) in [
                    (
                        "tb.zoom_mode",
                        "Zoom mode toggle",
                        s.toolbar.zoom_mode_button,
                    ),
                    ("tb.zoom_reset", "Reset zoom", s.toolbar.zoom_reset_button),
                    ("tb.zoom_presets", "Zoom presets", s.toolbar.zoom_presets),
                    ("tb.rotate", "Rotation controls", s.toolbar.rotate_buttons),
                    ("tb.flip", "Mirror controls", s.toolbar.flip_buttons),
                    ("tb.tags", "Tags dropdown", s.toolbar.tags_button),
                    (
                        "tb.delete",
                        "Delete to recycle bin",
                        s.toolbar.delete_button,
                    ),
                    ("tb.copy", "Copy to…", s.toolbar.copy_button),
                    ("tb.move", "Move to…", s.toolbar.move_button),
                    ("tb.explorer", "Show in Explorer", s.toolbar.explorer_button),
                    (
                        "tb.external",
                        "Open in default app",
                        s.toolbar.open_external_button,
                    ),
                    (
                        "tb.filmstrip",
                        "Filmstrip toggle",
                        s.toolbar.filmstrip_toggle,
                    ),
                ] {
                    b = b.child(self.toggle_button(k, n, v, cx));
                }
                b = b
                    .child(self.toggle_button(
                        "filmstrip",
                        "Show filmstrip by default",
                        s.show_filmstrip,
                        cx,
                    ))
                    .child(self.toggle_button(
                        "pos.locked",
                        "Lock floating toolbar position",
                        s.toolbar_pos.locked,
                        cx,
                    ))
                    .child(div().child(format!(
                        "Toolbar offset: {:.0}, {:.0} px",
                        s.toolbar_pos.offset_x, s.toolbar_pos.offset_y
                    )));
            }
            Tab::Shortcuts => {
                b = b.child(self.section("Tag sidebar modifier"));
                for opt in [
                    ExcludeModifier::None,
                    ExcludeModifier::Ctrl,
                    ExcludeModifier::Shift,
                    ExcludeModifier::Alt,
                ] {
                    let selected = s.exclude_modifier == opt;
                    b = b.child(self.enum_button(
                        "exclude-modifier",
                        format!("{}{}", if selected { "● " } else { "" }, opt.label()),
                        move |s| s.exclude_modifier = opt,
                        cx,
                    ));
                }
                b = b
                    .child(div().child(format!("Open selected media: {}", s.open_key)))
                    .child(div().child(
                        "Ratings 0–5 use number-row keys; preview stars set rating by mouse.",
                    ));
                b = b.child(self.section("Tag hotkeys F1–F9"));
                for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                    let val = s
                        .tag_hotkeys
                        .iter()
                        .find(|h| h.key == key)
                        .map(|h| format!("tag #{}", h.tag_id))
                        .unwrap_or_else(|| "None".into());
                    b = b.child(div().child(format!("{key}: {val}")));
                }
                b=b.child(div().child("Key capture and tag picker need tag tree and keybinding integration; stored assignments are preserved."));
            }
            Tab::Tags => {
                b = b.child(self.section("Global search scope"));
                for (k, n, v) in [
                    ("search.files", "Filenames", s.search_scopes.filenames),
                    ("search.paths", "Paths", s.search_scopes.paths),
                    ("search.tags", "Tags", s.search_scopes.tags),
                ] {
                    b = b.child(self.toggle_button(k, n, v, cx));
                }
                b = b.child(self.toggle_button(
                    "persist",
                    "Keep tag search query when switching tabs",
                    s.tag_search_persist,
                    cx,
                ));
                for opt in [TagSearchMatch::Contains, TagSearchMatch::StartsWith] {
                    let chosen = s.tag_search_match == opt;
                    b = b.child(self.enum_button(
                        "tag-match",
                        format!("{}{}", if chosen { "● " } else { "" }, opt.label()),
                        move |s| s.tag_search_match = opt,
                        cx,
                    ));
                }
                b = b.child(self.section("Source tag children after merge"));
                for opt in [TagMergeChildren::Reparent, TagMergeChildren::Delete] {
                    let chosen = s.tag_merge_children == opt;
                    b = b.child(self.enum_button(
                        "tag-merge",
                        format!("{}{}", if chosen { "● " } else { "" }, opt.label()),
                        move |s| s.tag_merge_children = opt,
                        cx,
                    ));
                }
            }
            Tab::Ai => {
                b = b
                    .child(self.section("AI analysis"))
                    .child(self.toggle_button(
                        "ai",
                        "Automatically analyze opened photos",
                        s.ai.auto_analyze,
                        cx,
                    ))
                    .child(self.section("Mandatory tag groups"));
                for (i, g) in s.ai.mandatory_groups.iter().enumerate() {
                    b = b
                        .child(div().child(format!("{}: {}", g.name, g.tags.join(", "))))
                        .child(
                            Button::new(format!("settings-group-remove-{i}"))
                                .label("Remove group")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.edit(
                                        |s| {
                                            if i < s.ai.mandatory_groups.len() {
                                                s.ai.mandatory_groups.remove(i);
                                            }
                                        },
                                        cx,
                                    )
                                })),
                        );
                }
                b=b.child(Button::new("settings-group-add").label("+ Add group").on_click(cx.listener(|this,_,_,cx|this.edit(|s|s.ai.mandatory_groups.push(AiTagGroup{name:format!("Group {}",s.ai.mandatory_groups.len()+1),tags:vec![]}),cx)))).child(div().child("Group name editing, tag selection and tag removal require the navigation tag-picker integration."));
            }
            Tab::Updates => {
                b=b.child(self.section("Updates")).child(self.toggle_button("updates","Check for updates on startup",s.auto_check_updates,cx)).child(div().child("Live update check/download/install actions are owned by the updater plugin."));
            }
        }
        b
    }
}
impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut r = div()
            .v_flex()
            .w_full()
            .h_full()
            .gap_2()
            .p_3()
            .bg(rgb(0x20232b))
            .text_color(rgb(0xe7e9ee))
            .child(div().text_lg().font_semibold().child("Settings"))
            .child(self.tabs(cx));
        match self.service.snapshot() {
            Ok(s) => {
                r = r.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .child(self.render_tab(&s, cx)),
                )
            }
            Err(e) => r = r.child(e.to_string()),
        }
        if let Some(e) = &self.error {
            r = r.child(
                div()
                    .text_color(rgb(0xff7070))
                    .child(format!("Could not save: {e}")),
            );
        }
        r
    }
}
