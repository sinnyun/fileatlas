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
#[path = "view/appearance.rs"]
mod appearance;
#[path = "view/capture.rs"]
mod capture;
#[path = "view/checkbox.rs"]
mod checkbox;
#[path = "view/design.rs"]
mod design;
#[path = "view/layout.rs"]
mod layout;
#[path = "view/plugins.rs"]
mod plugins;
#[path = "view/rail_entry.rs"]
mod rail_entry;
#[path = "view/shortcuts.rs"]
mod shortcuts;
#[path = "view/software.rs"]
mod software;
#[path = "view/tabs.rs"]
mod tabs;
#[path = "view/tag_choices.rs"]
mod tag_choices;
#[path = "view/tag_hotkeys.rs"]
mod tag_hotkeys;
#[path = "view/updates.rs"]
mod updates;
#[path = "view/updates_page.rs"]
mod updates_page;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Appearance,
    Shortcuts,
    Tags,
    Ai,
    Updates,
    Software,
    Plugins,
    PluginSettings,
}

pub(super) fn rail_entry(
    service: Arc<SettingsService>,
    surfaces: Arc<UiSurfaceState>,
) -> UiViewContribution {
    rail_entry::contribution(service, surfaces)
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
            let content = app
                .new(|cx| {
                    let mut view = SettingsView {
                        locale_revision: u64::MAX,
                        render_count: 0,
                        language_views: Vec::new(),
                        plugin_settings_views: Vec::new(),
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
                        update_progress_seen: photag_ui_bridge::refresh::ChangeGate::new(0),
                        update_status: None,
                        update_error: None,
                        update_offer_open: false,
                        update_offer_dismissed: false,
                        confirm_install: false,
                        group_name_input: group_name_input.clone(),
                        editing_group: None,
                        tab: Tab::Appearance,
                        selected_plugin_id: None,
                        error: None,
                        capturing_open_key: false,
                        capturing_shortcut: None,
                        shortcut_error: None,
                        focus_handle: cx.focus_handle(),
                        surface_visible: view_surfaces.is_open("settings"),
                        tag_choices: Default::default(),
                        software_check: Default::default(),
                        software_revision: 0,
                        software_tab_active: false,
                    };
                    let surface_poll = Arc::clone(&view_surfaces);
                    let mut surface_gate = photag_ui_bridge::refresh::ChangeGate::new((
                        surface_poll.is_open("settings"),
                        surface_poll.value("settings.tab"),
                        surface_poll.value("settings.plugin_id"),
                    ));
                    let mut provider_gate =
                        photag_ui_bridge::refresh::ChangeGate::new(Vec::<(String, u64)>::new());
                    let mut runtime_gate = photag_ui_bridge::refresh::ChangeGate::new(Vec::<
                        crate::PluginRuntimeStatus,
                    >::new(
                    ));
                    photag_ui_bridge::refresh::poll(
                        cx,
                        std::time::Duration::from_millis(100),
                        move |view: &mut SettingsView, cx| {
                            let visible = surface_poll.is_open("settings");
                            let changed = surface_gate.changed((
                                visible,
                                surface_poll.value("settings.tab"),
                                surface_poll.value("settings.plugin_id"),
                            ));
                            view.surface_visible = visible;
                            if (!visible || changed)
                                && (view.capturing_shortcut.is_some() || view.capturing_open_key)
                            {
                                view.capturing_open_key = false;
                                view.cancel_shortcut_capture();
                            }
                            let providers = view
                                .cordis
                                .try_service::<photag_ui_bridge::UiBridgeService>()
                                .ok()
                                .and_then(|bridge| bridge.views.snapshot().ok())
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|entry| {
                                    entry.contribution.settings_page.is_some()
                                        || entry.contribution.slot == "settings.language"
                                })
                                .map(|entry| (entry.contribution.id, entry.generation))
                                .collect::<Vec<_>>();
                            let mut changed = view.poll_tag_choices()
                                | view.poll_software_check()
                                | changed
                                | provider_gate.changed(providers)
                                | runtime_gate
                                    .changed(view.service.plugin_runtime().snapshot_now());
                            if surface_poll.is_open("settings.check-updates") {
                                let _ = surface_poll.set_open("settings.check-updates", false);
                                if view.update_check.is_none() {
                                    view.begin_update_check(true);
                                    view.monitor_updates(cx);
                                    changed = true;
                                }
                            }
                            changed.into()
                        },
                    );
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
                (1120., 840.),
            );
            options.min_size = (920., 580.);
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
    render_count: u64,
    language_views: Vec<(String, u64, AnyView)>,
    plugin_settings_views: Vec<(String, String, String, String, u64, AnyView)>,
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
    update_progress_seen: photag_ui_bridge::refresh::ChangeGate<u64>,
    update_status: Option<String>,
    update_error: Option<String>,
    update_offer_open: bool,
    update_offer_dismissed: bool,
    update_check_manual: bool,
    confirm_install: bool,
    group_name_input: Entity<InputState>,
    editing_group: Option<usize>,
    tab: Tab,
    selected_plugin_id: Option<String>,
    error: Option<String>,
    capturing_open_key: bool,
    capturing_shortcut: Option<String>,
    shortcut_error: Option<String>,
    focus_handle: FocusHandle,
    surface_visible: bool,
    tag_choices: photag_ui_bridge::read_model::ReadModel<(bool, u64), Vec<tag_choices::TagChoice>>,
    software_check: photag_ui_bridge::read_model::ReadModel<u64, Vec<software::SoftwareStatus>>,
    software_revision: u64,
    software_tab_active: bool,
}
impl SettingsView {
    fn close_surface(&mut self, cx: &mut Context<Self>) {
        self.capturing_open_key = false;
        self.cancel_shortcut_capture();
        let _ = self.surfaces.set_capturing_key("settings", false);
        self.surface_visible = false;
        if let Err(error) = self.surfaces.set_open("settings", false) {
            self.error = Some(format!("Could not close Settings: {error}"));
        }
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.capturing_shortcut.is_some() {
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                let _ = self.surfaces.suppress_next_escape("settings");
            }
            self.capture_shortcut(event, cx);
        } else if self.capturing_open_key {
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                let _ = self.surfaces.suppress_next_escape("settings");
            }
            self.capture_open_key(event, cx);
        } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.close_surface(cx);
        }
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
                photag_ui_bridge::theme::set_color_mode(if settings.color_mode == "light" {
                    photag_ui_bridge::theme::ColorMode::Light
                } else {
                    photag_ui_bridge::theme::ColorMode::Dark
                });
                photag_ui_bridge::theme::apply_accent_components(cx);
                // Paint-time tokens (accent, region separation) live in every region, so
                // repaint all windows instead of waiting for each view's next notify.
                cx.refresh_windows();
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
                "search.time" => s.search_scopes.time = !s.search_scopes.time,
                _ => {}
            },
            cx,
        );
    }
    fn section(&self, label: &'static str) -> Div {
        design::heading(label)
    }
}

impl SettingsView {
    fn sync_language_views(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entries = self
            .cordis
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .ok()
            .and_then(|bridge| bridge.views.snapshot().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                entry.contribution.slot == "settings.language"
                    || entry.contribution.settings_page.is_some()
            })
            .collect::<Vec<_>>();
        let plugin_entries = entries
            .iter()
            .filter(|entry| entry.contribution.settings_page.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let mut previous_plugins = std::mem::take(&mut self.plugin_settings_views);
        self.plugin_settings_views = plugin_entries
            .into_iter()
            .filter_map(|entry| {
                let meta = entry.contribution.settings_page.as_ref()?;
                let id = entry.contribution.id.clone();
                let generation = entry.generation;
                let view = previous_plugins
                    .iter()
                    .position(|(_, _, _, old_id, old_generation, _)| {
                        old_id == &id && *old_generation == generation
                    })
                    .map(|index| previous_plugins.remove(index).5)
                    .unwrap_or_else(|| (entry.contribution.factory)(window, cx));
                Some((
                    entry.owner,
                    meta.title.clone(),
                    meta.description.clone(),
                    id,
                    generation,
                    view,
                ))
            })
            .collect();
        let mut previous = std::mem::take(&mut self.language_views);
        self.language_views = entries
            .into_iter()
            .filter(|entry| entry.contribution.slot == "settings.language")
            .map(|entry| {
                let id = entry.contribution.id;
                let generation = entry.generation;
                let view = previous
                    .iter()
                    .position(|(old_id, old_generation, _)| {
                        old_id == &id && *old_generation == generation
                    })
                    .map(|index| previous.remove(index).2)
                    .unwrap_or_else(|| (entry.contribution.factory)(window, cx));
                (id, generation, view)
            })
            .collect();
    }
}
