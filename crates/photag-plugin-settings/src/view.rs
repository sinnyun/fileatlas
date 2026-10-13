//! GPUI settings surface migrated from the pinned PhoTag settings dialog.
use crate::{ExcludeModifier, Settings, SettingsService, TagMergeChildren, TagSearchMatch};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_plugin_navigation::NavigationService;
use photag_plugin_session::SessionService;
use photag_plugin_updater::{UpdateInfo, UpdaterService};
use photag_ui_bridge::{UiViewContribution, theme};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;

#[path = "view/ai_groups.rs"]
mod ai_groups;
#[path = "view/tag_hotkeys.rs"]
mod tag_hotkeys;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Appearance,
    Toolbar,
    Shortcuts,
    Tags,
    Ai,
    Updates,
}
pub(super) fn settings(
    service: Arc<SettingsService>,
    navigation: Option<Weak<NavigationService>>,
    updater: Option<Arc<UpdaterService>>,
    session: Option<Arc<SessionService>>,
) -> UiViewContribution {
    UiViewContribution::new(
        "settings/panel-view",
        "workspace.settings",
        0,
        move |window, app| {
            let group_name_input =
                app.new(|cx| InputState::new(window, cx).placeholder("Group name"));
            app.new(|cx| {
                let mut view = SettingsView {
                    service: Arc::clone(&service),
                    navigation: navigation.clone(),
                    updater: updater.clone(),
                    session: session.clone(),
                    update_check: None,
                    update_download: None,
                    update_info: None,
                    update_staged: None,
                    update_progress: Arc::new(AtomicU64::new(0)),
                    update_status: None,
                    update_error: None,
                    confirm_install: false,
                    group_name_input: group_name_input.clone(),
                    editing_group: None,
                    tab: Tab::Appearance,
                    error: None,
                    capturing_open_key: false,
                    focus_handle: cx.focus_handle(),
                };
                let auto_check = service
                    .snapshot()
                    .map(|settings| settings.auto_check_updates)
                    .unwrap_or(false);
                if auto_check && view.updater.is_some() {
                    view.begin_update_check(false);
                    view.monitor_updates(cx);
                }
                view
            })
            .into()
        },
    )
}
struct SettingsView {
    service: Arc<SettingsService>,
    navigation: Option<Weak<NavigationService>>,
    updater: Option<Arc<UpdaterService>>,
    session: Option<Arc<SessionService>>,
    update_check: Option<mpsc::Receiver<Result<Option<UpdateInfo>, String>>>,
    update_download: Option<mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    update_info: Option<UpdateInfo>,
    update_staged: Option<std::path::PathBuf>,
    update_progress: Arc<AtomicU64>,
    update_status: Option<String>,
    update_error: Option<String>,
    confirm_install: bool,
    group_name_input: Entity<InputState>,
    editing_group: Option<usize>,
    tab: Tab,
    error: Option<String>,
    capturing_open_key: bool,
    focus_handle: FocusHandle,
}
impl SettingsView {
    fn begin_update_check(&mut self, manual: bool) {
        let Some(updater) = &self.updater else {
            self.update_error = Some("Updater service is unavailable.".into());
            return;
        };
        self.update_error = None;
        self.update_status = Some("Checking GitHub for a newer release…".into());
        self.update_check = Some(updater.check_async());
        if manual {
            self.update_info = None;
        }
    }

    fn begin_update_download(&mut self) {
        let (Some(updater), Some(info)) = (&self.updater, self.update_info.clone()) else {
            return;
        };
        self.update_error = None;
        self.update_status = Some(format!("Downloading PhoTag {}…", info.version));
        self.update_progress.store(0, Ordering::Relaxed);
        self.update_download =
            Some(updater.download_async(info, Arc::clone(&self.update_progress)));
    }

    fn monitor_updates(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                let active = this
                    .update(cx, |view, cx| {
                        view.poll_updates();
                        cx.notify();
                        view.update_check.is_some() || view.update_download.is_some()
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
            }
        })
        .detach();
    }

    fn poll_updates(&mut self) {
        let check_result = self
            .update_check
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = check_result {
            self.update_check = None;
            match result {
                Ok(Some(info)) => {
                    self.update_status = Some(format!(
                        "PhoTag {} is available ({:.1} MB).",
                        info.version,
                        info.size_mb()
                    ));
                    self.update_info = Some(info);
                }
                Ok(None) => {
                    self.update_info = None;
                    self.update_status = Some(format!(
                        "PhoTag {} is up to date.",
                        photag_plugin_updater::current_version()
                    ));
                }
                Err(error) => {
                    self.update_status = None;
                    self.update_error = Some(error);
                }
            }
        }
        let download_result = self
            .update_download
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = download_result {
            self.update_download = None;
            match result {
                Ok(path) => {
                    self.update_staged = Some(path);
                    self.confirm_install = false;
                    self.update_status =
                        Some("Download verified. PhoTag is ready to install and restart.".into());
                }
                Err(error) => {
                    self.update_status = None;
                    self.update_error = Some(error);
                }
            }
        }
    }

    fn install_update(&mut self, cx: &mut Context<Self>) {
        if !self.confirm_install {
            self.confirm_install = true;
            self.update_status = Some("Confirm restart and install the verified update.".into());
            cx.notify();
            return;
        }
        let Some(path) = self.update_staged.clone() else {
            return;
        };
        let session_save = self
            .session
            .as_ref()
            .ok_or_else(|| {
                "Session service is unavailable; update installation was stopped.".to_owned()
            })
            .and_then(|session| {
                let snapshot = session.snapshot().map_err(|error| error.to_string())?;
                session.replace(snapshot).map_err(|error| error.to_string())
            });
        if let Err(error) = session_save {
            self.update_error = Some(error);
            cx.notify();
            return;
        }
        let result = self
            .updater
            .as_ref()
            .ok_or_else(|| "Updater service is unavailable.".to_owned())
            .and_then(|updater| updater.schedule_install(path.clone()));
        match result {
            Ok(()) => {
                self.update_staged = None;
                self.update_status = Some("Closing PhoTag to install the verified update…".into());
                cx.quit();
            }
            Err(error) => {
                let log = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("PhoTag")
                    .join("update-error.txt");
                let _ = std::fs::create_dir_all(
                    log.parent().unwrap_or_else(|| std::path::Path::new(".")),
                );
                let _ = std::fs::write(
                    &log,
                    format!(
                        "PhoTag {} could not install update from {}\n\n{}\n",
                        photag_plugin_updater::current_version(),
                        path.display(),
                        error
                    ),
                );
                self.update_error = Some(format!("{error} (details: {})", log.display()));
            }
        }
        cx.notify();
    }

    fn capture_open_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if !self.capturing_open_key {
            return;
        }

        let key = match event.keystroke.key.to_ascii_lowercase().as_str() {
            "escape" => {
                self.capturing_open_key = false;
                cx.notify();
                return;
            }
            "space" | " " => "Space".to_owned(),
            "enter" => "Enter".to_owned(),
            "tab" => "Tab".to_owned(),
            "backspace" => "Backspace".to_owned(),
            "delete" => "Delete".to_owned(),
            "insert" => "Insert".to_owned(),
            "home" => "Home".to_owned(),
            "end" => "End".to_owned(),
            "pageup" => "PageUp".to_owned(),
            "pagedown" => "PageDown".to_owned(),
            "left" => "ArrowLeft".to_owned(),
            "right" => "ArrowRight".to_owned(),
            "up" => "ArrowUp".to_owned(),
            "down" => "ArrowDown".to_owned(),
            key if key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric() => {
                key.to_ascii_uppercase()
            }
            key if key.len() >= 2 && key.starts_with('f') && key[1..].parse::<u8>().is_ok() => {
                key.to_ascii_uppercase()
            }
            _ => {
                self.error = Some("That key cannot be saved as a PhoTag shortcut.".into());
                self.capturing_open_key = false;
                cx.notify();
                return;
            }
        };

        self.edit(|settings| settings.open_key = key.clone(), cx);
        self.capturing_open_key = false;
        cx.notify();
    }

    fn open_key_capture_button(&self, cx: &mut Context<Self>) -> Button {
        Button::new("settings-open-key-capture")
            .label(if self.capturing_open_key {
                "Press a key… (Escape cancels)".to_owned()
            } else {
                "Change key".to_owned()
            })
            .on_click(cx.listener(|this, _, window, cx| {
                this.capturing_open_key = !this.capturing_open_key;
                this.error = None;
                window.focus(&this.focus_handle, cx);
                cx.notify();
            }))
    }

    fn edit(&mut self, f: impl Fn(&mut Settings), cx: &mut Context<Self>) {
        match self.service.update(f) {
            Ok(settings) => {
                photag_ui_bridge::theme::set_accent_name(&settings.accent_name);
                self.error = None;
            }
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
        let action = Arc::new(action);
        Button::new(format!("settings-{id}"))
            .label(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                let action = Arc::clone(&action);
                this.edit(move |settings| action(settings), cx);
            }))
    }

    fn section(&self, label: &'static str) -> Div {
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
                    )))
                    .child(div().child("Adjust the preview toolbar position in 10 px steps."))
                    .child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(self.enum_button(
                                "pos-left",
                                "← 10 px".into(),
                                |s| s.toolbar_pos.offset_x -= 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-right",
                                "10 px →".into(),
                                |s| s.toolbar_pos.offset_x += 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-up",
                                "↑ 10 px".into(),
                                |s| s.toolbar_pos.offset_y -= 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-down",
                                "10 px ↓".into(),
                                |s| s.toolbar_pos.offset_y += 10.0,
                                cx,
                            ))
                            .child(self.enum_button(
                                "pos-reset",
                                "Reset position".into(),
                                |s| {
                                    s.toolbar_pos.offset_x = 0.0;
                                    s.toolbar_pos.offset_y = 0.0;
                                },
                                cx,
                            )),
                    );
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
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_2()
                            .child(format!("Open selected media: {}", s.open_key))
                            .child(self.open_key_capture_button(cx)),
                    )
                    .child(div().child(
                        "Ratings 0–5 use number-row keys; preview stars set rating by mouse.",
                    ));
                b = b.child(self.section("Tag hotkeys F1–F9"));
                b =
                    b.child(div().child(
                        "Press an F-key in the gallery or preview to toggle its assigned tag.",
                    ));
                match self
                    .navigation
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .map(|navigation| navigation.snapshot())
                {
                    Some(Ok(snapshot)) => {
                        let mut flat_tags = Vec::new();
                        let mut pending: Vec<_> = snapshot.tags.into_iter().rev().collect();
                        while let Some(tag) = pending.pop() {
                            let icon = tag
                                .icon
                                .as_deref()
                                .filter(|icon| !icon.is_empty())
                                .unwrap_or("🏷");
                            flat_tags.push((
                                tag.id,
                                format!("{icon} {}", tag.full_path.replace('>', " > ")),
                            ));
                            pending.extend(tag.children.into_iter().rev());
                        }
                        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                            let selected = s
                                .tag_hotkeys
                                .iter()
                                .find(|hotkey| hotkey.key == key)
                                .map(|hotkey| hotkey.tag_id);
                            b = b.child(tag_hotkeys::button(
                                key,
                                selected,
                                flat_tags.clone(),
                                cx.entity().downgrade(),
                            ));
                        }
                    }
                    Some(Err(error)) => {
                        b = b.child(div().child(format!("Could not load tags: {error}")));
                    }
                    None => {
                        b = b.child(div().child(
                            "Tag choices are unavailable while the navigation plugin is stopped.",
                        ));
                        for key in ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9"] {
                            let selected = s
                                .tag_hotkeys
                                .iter()
                                .find(|hotkey| hotkey.key == key)
                                .map(|hotkey| hotkey.tag_id);
                            let assigned = selected
                                .map(|id| format!("tag #{id}"))
                                .unwrap_or_else(|| "None".into());
                            b = b.child(div().child(format!("{key}: {assigned}")));
                        }
                    }
                }
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
                b = b.child(self.ai_groups(s, cx));
            }
            Tab::Updates => {
                b = b
                    .child(self.section("Updates"))
                    .child(div().child(format!(
                        "PhoTag {}",
                        photag_plugin_updater::current_version()
                    )))
                    .child(self.toggle_button(
                        "updates",
                        "Check for updates on startup",
                        s.auto_check_updates,
                        cx,
                    ));
                if self.updater.is_some() {
                    let checking = self.update_check.is_some();
                    b = b.child(
                        Button::new("settings-update-check")
                            .label(if checking { "Checking…" } else { "Check now" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.update_check.is_none() {
                                    this.begin_update_check(true);
                                    this.monitor_updates(cx);
                                }
                            })),
                    );
                    if let Some(status) = &self.update_status {
                        b = b.child(div().child(status.clone()));
                    }
                    if let Some(error) = &self.update_error {
                        b = b.child(
                            div()
                                .text_color(rgb(theme::DANGER))
                                .child(format!("Update failed: {error}")),
                        );
                    }
                    if let Some(info) = &self.update_info {
                        b = b
                            .child(div().child(format!("Release notes\n{}", info.notes_plain())))
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("Release page: {}", info.html_url)),
                            );
                        if self.update_download.is_some() {
                            let got = self.update_progress.load(Ordering::Relaxed);
                            let total = info.asset_size;
                            b = b.child(div().child(format!(
                                "Downloading: {:.1} / {:.1} MB ({:.0}%)",
                                got as f64 / 1_048_576.0,
                                info.size_mb(),
                                if total == 0 {
                                    0.0
                                } else {
                                    got as f64 * 100.0 / total as f64
                                }
                            )));
                        } else if self.update_staged.is_none() {
                            b = b.child(
                                Button::new("settings-update-download")
                                    .label("Download and verify update")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if this.update_download.is_none() {
                                            this.begin_update_download();
                                            this.monitor_updates(cx);
                                        }
                                    })),
                            );
                        }
                    }
                    if self.update_staged.is_some() {
                        b = b.child(
                            Button::new("settings-update-install")
                                .label(if self.confirm_install {
                                    "Confirm restart and install"
                                } else {
                                    "Restart and install…"
                                })
                                .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                        );
                        if self.confirm_install {
                            b = b.child(div().child("PhoTag will replace its executable and launch the updated version."));
                        }
                    }
                } else {
                    b = b.child(div().child("Updater service is unavailable."));
                }
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
            .bg(rgb(theme::SURFACE_BASE))
            .text_color(rgb(theme::TEXT_PRIMARY))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, _, cx| this.capture_open_key(event, cx)))
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
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Could not save: {e}")),
            );
        }
        r
    }
}
