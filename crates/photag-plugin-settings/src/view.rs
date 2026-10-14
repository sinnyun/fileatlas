//! GPUI settings surface migrated from the pinned PhoTag settings dialog.
use crate::{ExcludeModifier, Settings, SettingsService, TagMergeChildren, TagSearchMatch};
use cordis::{Context as CordisContext, Routing};
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::*;
use photag_contracts::TagInteractionPreferencesChanged;
use photag_plugin_navigation::NavigationService;
use photag_plugin_session::SessionService;
use photag_plugin_updater::{UpdateInfo, UpdaterService};
use photag_ui_bridge::{UiSurfaceState, UiViewContribution, theme};
use std::sync::{Arc, Weak, atomic::AtomicU64, mpsc};

#[path = "view/ai_groups.rs"]
mod ai_groups;
#[path = "view/checkbox.rs"]
mod checkbox;
#[path = "view/tabs.rs"]
mod tabs;
#[path = "view/tag_hotkeys.rs"]
mod tag_hotkeys;
#[path = "view/toolbar.rs"]
mod toolbar;
#[path = "view/updates.rs"]
mod updates;
#[path = "view/updates_page.rs"]
mod updates_page;

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
    surfaces: Arc<UiSurfaceState>,
    cordis: CordisContext,
) -> UiViewContribution {
    UiViewContribution::new(
        "settings/panel-view",
        "workspace.modal",
        0,
        move |window, app| {
            let view_cordis = cordis.clone();
            let view_surfaces = Arc::clone(&surfaces);
            let group_name_input = app.new(|cx| {
                InputState::new(window, cx).placeholder(photag_ui_bridge::i18n::tr("Group name"))
            });
            let language_views = view_cordis
                .try_service::<photag_ui_bridge::UiBridgeService>()
                .ok()
                .and_then(|bridge| bridge.views.snapshot().ok())
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| entry.contribution.slot == "settings.language")
                .map(|entry| (entry.contribution.factory)(window, app))
                .collect::<Vec<_>>();
            let content = app
                .new(|cx| {
                    let mut view = SettingsView {
                        locale_revision: u64::MAX,
                        language_views,
                        service: Arc::clone(&service),
                        surfaces: Arc::clone(&view_surfaces),
                        cordis: view_cordis,
                        navigation: navigation.clone(),
                        updater: updater.clone(),
                        session: session.clone(),
                        update_check: None,
                        update_check_manual: false,
                        update_download: None,
                        update_info: None,
                        update_staged: None,
                        update_progress: Arc::new(AtomicU64::new(0)),
                        update_status: None,
                        update_error: None,
                        update_offer_open: false,
                        update_offer_dismissed: false,
                        confirm_install: false,
                        group_name_input: group_name_input.clone(),
                        editing_group: None,
                        // Pinned main.rs:130 starts Settings on Toolbar.
                        tab: Tab::Toolbar,
                        error: None,
                        capturing_open_key: false,
                        focus_handle: cx.focus_handle(),
                        surface_visible: view_surfaces.is_open("settings"),
                    };
                    let surface_poll = Arc::clone(&view_surfaces);
                    cx.spawn(async move |this, cx| {
                        let mut previous = surface_poll.is_open("settings");
                        let mut previous_tab = surface_poll.value("settings.tab");
                        loop {
                            cx.background_executor()
                                .timer(std::time::Duration::from_millis(100))
                                .await;
                            let visible = surface_poll.is_open("settings");
                            let tab = surface_poll.value("settings.tab");
                            if surface_poll.is_open("settings.check-updates") {
                                let _ = surface_poll.set_open("settings.check-updates", false);
                                if this
                                    .update(cx, |view: &mut SettingsView, cx| {
                                        if view.update_check.is_none() {
                                            view.begin_update_check(true);
                                            view.monitor_updates(cx);
                                        }
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            if visible != previous || tab != previous_tab {
                                previous_tab = tab;
                                previous = visible;
                                if this
                                    .update(cx, |view: &mut SettingsView, cx| {
                                        view.surface_visible = visible;
                                        cx.notify();
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    })
                    .detach();
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
                .into();
            let visible = surfaces.clone();
            let close = surfaces.clone();
            let mut options = photag_ui_bridge::panels::PanelOptions::new(
                "settings",
                "PhoTag · Settings",
                (900., 680.),
            );
            options.min_size = (680., 360.);
            photag_ui_bridge::panels::mount(
                content,
                options,
                surfaces.clone(),
                move || visible.is_open("settings"),
                move || {
                    let _ = close.set_open("settings", false);
                },
                app,
            )
        },
    )
    .with_panel_id("settings")
}
pub(super) struct SettingsView {
    locale_revision: u64,
    language_views: Vec<AnyView>,
    service: Arc<SettingsService>,
    surfaces: Arc<UiSurfaceState>,
    cordis: CordisContext,
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
    update_offer_open: bool,
    update_offer_dismissed: bool,
    update_check_manual: bool,
    confirm_install: bool,
    group_name_input: Entity<InputState>,
    editing_group: Option<usize>,
    tab: Tab,
    error: Option<String>,
    capturing_open_key: bool,
    focus_handle: FocusHandle,
    surface_visible: bool,
}
impl SettingsView {
    fn close_surface(&mut self, cx: &mut Context<Self>) {
        self.capturing_open_key = false;
        let _ = self.surfaces.set_capturing_key("settings", false);
        self.surface_visible = false;
        if let Err(error) = self.surfaces.set_open("settings", false) {
            self.error = Some(format!("Could not close Settings: {error}"));
        }
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.capturing_open_key {
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                let _ = self.surfaces.suppress_next_escape("settings");
            }
            self.capture_open_key(event, cx);
        } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.close_surface(cx);
        }
    }

    fn capture_open_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if !self.capturing_open_key {
            return;
        }

        let key = match event.keystroke.key.to_ascii_lowercase().as_str() {
            "escape" => {
                self.capturing_open_key = false;
                let _ = self.surfaces.set_capturing_key("settings", false);
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
                let _ = self.surfaces.set_capturing_key("settings", false);
                cx.notify();
                return;
            }
        };

        self.edit(|settings| settings.open_key = key.clone(), cx);
        self.capturing_open_key = false;
        let _ = self.surfaces.set_capturing_key("settings", false);
        cx.notify();
    }

    fn open_key_capture_button(&self, cx: &mut Context<Self>) -> Button {
        Button::new("settings-open-key-capture")
            .label(photag_ui_bridge::i18n::tr(
                &(if self.capturing_open_key {
                    "Press a key… (Escape cancels)".to_owned()
                } else {
                    "Change key".to_owned()
                })
                .to_string(),
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.capturing_open_key = !this.capturing_open_key;
                if let Err(error) = this
                    .surfaces
                    .set_capturing_key("settings", this.capturing_open_key)
                {
                    this.error = Some(format!("Could not update shortcut capture state: {error}"));
                }
                this.error = None;
                window.focus(&this.focus_handle, cx);
                cx.notify();
            }))
    }

    fn edit(&mut self, f: impl Fn(&mut Settings), cx: &mut Context<Self>) {
        let old_preferences = self.service.snapshot().ok().map(|settings| {
            (
                settings.exclude_modifier,
                settings.tag_search_match,
                settings.tag_search_persist,
                settings.tag_merge_children,
            )
        });
        match self.service.update(f) {
            Ok(settings) => {
                photag_ui_bridge::theme::set_accent_name(&settings.accent_name);
                photag_ui_bridge::theme::apply_accent_components(cx);
                let new_preferences = (
                    settings.exclude_modifier,
                    settings.tag_search_match,
                    settings.tag_search_persist,
                    settings.tag_merge_children,
                );
                if old_preferences != Some(new_preferences) {
                    let cordis = self.cordis.clone();
                    let preferences = crate::interaction_preferences(&settings);
                    cx.spawn(async move |_, _| {
                        let _ = cordis
                            .emit::<TagInteractionPreferencesChanged>(
                                Routing::Unscoped,
                                preferences,
                            )
                            .await;
                    })
                    .detach();
                }
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
    fn enum_button(
        &self,
        id: impl Into<String>,
        label: String,
        action: impl Fn(&mut Settings) + Send + Sync + 'static,
        cx: &mut Context<Self>,
    ) -> Button {
        let action = Arc::new(action);
        Button::new(format!("settings-{}", id.into()))
            .label(photag_ui_bridge::i18n::tr(&(label).to_string()))
            .on_click(cx.listener(move |this, _, _, cx| {
                let action = Arc::clone(&action);
                this.edit(move |settings| action(settings), cx);
            }))
    }

    fn section(&self, label: &'static str) -> Div {
        div()
            .pt_2()
            .font_semibold()
            .child(photag_ui_bridge::i18n::tr(label))
    }
    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rail = div().v_flex().w(px(166.)).gap(px(6.)).p_2().flex_shrink_0();
        use gpui_kit::assets::IconName;
        for (tab, name, icon) in [
            (Tab::Appearance, "Appearance", IconName::Monitor),
            (Tab::Toolbar, "Toolbar", IconName::SlidersHorizontal),
            (Tab::Shortcuts, "Shortcuts", IconName::Keyboard),
            (Tab::Tags, "Tags", IconName::Tag),
            (Tab::Ai, "AI", IconName::Sparkles),
            (Tab::Updates, "Updates", IconName::RefreshCw),
        ] {
            let active = self.tab == tab;
            rail = rail.child(
                Button::new(format!("settings-tab-{name}"))
                    .ghost()
                    .w_full()
                    .h(px(38.))
                    .px(px(10.))
                    .justify_start()
                    .rounded(px(4.))
                    .border_l(px(3.))
                    .border_color(if active {
                        rgb(theme::accent())
                    } else {
                        rgba(0)
                    })
                    .bg(if active {
                        rgba((theme::accent() << 8) | 0x48)
                    } else {
                        rgba(0)
                    })
                    .text_color(rgb(theme::TEXT_PRIMARY))
                    .text_size(px(14.))
                    .child(
                        div()
                            .h_flex()
                            .w_full()
                            .items_center()
                            .justify_start()
                            .gap_2()
                            .child(gpui_kit::component::Icon::new(icon))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(14.))
                                    .text_color(rgb(theme::TEXT_PRIMARY))
                                    .child(photag_ui_bridge::i18n::tr(name)),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab;
                        let name = match tab {
                            Tab::Appearance => "appearance",
                            Tab::Toolbar => "toolbar",
                            Tab::Shortcuts => "shortcuts",
                            Tab::Tags => "tags",
                            Tab::Ai => "ai",
                            Tab::Updates => "updates",
                        };
                        let _ = this.surfaces.set_value("settings.tab", name);
                        cx.notify()
                    })),
            );
        }
        rail
    }
}
impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locale_revision = photag_ui_bridge::i18n::revision();
        if self.locale_revision != locale_revision {
            self.locale_revision = locale_revision;
            self.group_name_input.update(cx, |input, cx| {
                input.set_placeholder(photag_ui_bridge::i18n::tr("Group name"), window, cx)
            });
        }

        if !self.surface_visible {
            return self.render_update_offer(window, cx).into_any_element();
        }
        if let Some(tab) = self.surfaces.value("settings.tab") {
            self.tab = match tab.as_str() {
                "toolbar" => Tab::Toolbar,
                "shortcuts" => Tab::Shortcuts,
                "tags" => Tab::Tags,
                "ai" => Tab::Ai,
                "updates" => Tab::Updates,
                _ => Tab::Appearance,
            };
        }
        let bounds = photag_ui_bridge::panels::content_bounds(&self.surfaces, "settings", window);
        let card_width = bounds.width;
        let card_height = bounds.height;
        let body_height = (card_height - 44.).max(8.);
        let content_width = (card_width - 167.).max(1.);
        let mut card = div()
            .absolute()
            .inset_0()
            .v_flex()
            .w(px(card_width))
            .h(px(card_height))
            .max_h(px(card_height))
            .rounded_md()
            .bg(rgb(theme::SURFACE_MODAL))
            .text_color(rgb(theme::TEXT_PRIMARY))
            .border_1()
            .border_color(rgba(theme::HAIRLINE.to_rgba()))
            .track_focus(&self.focus_handle)
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .h(px(body_height))
                    .flex_shrink_0()
                    .child(
                        div()
                            .v_flex()
                            .w(px(166.))
                            .h(px(body_height))
                            .flex_shrink_0()
                            .bg(rgb(theme::SURFACE_BASE))
                            .child(self.tabs(cx)),
                    )
                    .child(
                        div()
                            .h(px(body_height))
                            .w(px(1.))
                            .flex_shrink_0()
                            .bg(rgba(theme::HAIRLINE.to_rgba())),
                    )
                    .child(
                        div()
                            .v_flex()
                            .w(px(content_width))
                            .h(px(body_height))
                            .flex_shrink_0()
                            .bg(rgb(theme::SURFACE_MODAL))
                            .child(div().size_full().overflow_y_scrollbar().p_4().child(
                                match self.service.snapshot() {
                                    Ok(s) => self.render_tab(&s, cx).into_any_element(),
                                    Err(error) => div().child(error.to_string()).into_any_element(),
                                },
                            )),
                    ),
            );
        card = card.child(
            div()
                .h_flex()
                .h(px(44.))
                .flex_shrink_0()
                .items_center()
                .justify_between()
                .border_t_1()
                .border_color(rgba(theme::HAIRLINE.to_rgba()))
                .bg(rgb(theme::SURFACE_MODAL))
                .px_3()
                .py_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_SECONDARY))
                        .child(photag_ui_bridge::i18n::tr(
                            "Settings changes are saved automatically.",
                        )),
                )
                .child(
                    Button::new("settings-close-footer")
                        .label(photag_ui_bridge::i18n::tr("Close"))
                        .on_click(cx.listener(|this, _, _, cx| this.close_surface(cx))),
                ),
        );
        if let Some(e) = &self.error {
            card = card.child(
                div()
                    .text_color(rgb(theme::DANGER))
                    .child(format!("Could not save: {e}")),
            );
        }
        let mut root = div()
            .absolute()
            .inset_0()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, _, cx| this.handle_key_down(event, cx)))
            .child(card);
        if self.update_offer_open {
            root = root.child(self.render_update_offer(window, cx));
        }
        root.into_any_element()
    }
}
