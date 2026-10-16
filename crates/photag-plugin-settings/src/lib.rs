//! Cordis-owned PhoTag settings persistence service.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use cordis::Routing;
use cordis::event::observer_sync;
use cordis::{Context, Plugin, Service};
use photag_contracts::{
    TagExcludeModifier, TagInteractionPreferences, TagInteractionPreferencesChanged,
};
use photag_ui_bridge::{
    ContributionKind, UiBridgeService, UiCommandInvoked, UiContribution, register_for_plugin,
    register_views_for_plugin,
};

pub mod settings;
pub use settings::*;

mod view;

/// Shared, mutable user preferences backed by the original settings JSON file.
pub struct SettingsService {
    state: RwLock<Settings>,
    path: PathBuf,
}

impl Service for SettingsService {
    const NAME: &'static str = "photag/settings";
}

impl SettingsService {
    /// Load settings using PhoTag's default-on-missing-or-invalid behavior.
    pub fn open_default() -> Self {
        Self::open_at(default_settings_path())
    }

    /// Load settings from an explicit path, preserving the original JSON shape.
    pub fn open_at(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            state: RwLock::new(state),
            path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn snapshot(&self) -> Result<Settings, SettingsServiceError> {
        self.state
            .read()
            .map(|state| state.clone())
            .map_err(|_| SettingsServiceError::Poisoned)
    }

    /// Replace preferences and persist them to the source-compatible file.
    pub fn replace(&self, settings: Settings) -> Result<(), SettingsServiceError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| SettingsServiceError::Poisoned)?;
        self.persist(&settings)?;
        *state = settings;
        Ok(())
    }

    /// Serialize the edit and disk publication; failed writes keep the previous snapshot.
    pub fn update(
        &self,
        edit: impl FnOnce(&mut Settings),
    ) -> Result<Settings, SettingsServiceError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| SettingsServiceError::Poisoned)?;
        let mut settings = state.clone();
        edit(&mut settings);
        self.persist(&settings)?;
        *state = settings.clone();
        Ok(settings)
    }

    fn apply_sidebar_update(
        &self,
        last: &std::sync::Mutex<u64>,
        update: photag_contracts::SidebarFoldersUpdate,
    ) -> Result<(), SettingsServiceError> {
        let mut last = last.lock().map_err(|_| SettingsServiceError::Poisoned)?;
        if update.revision > *last {
            self.update(|settings| settings.sidebar_folders = update.config)?;
            *last = update.revision;
        }
        Ok(())
    }

    /// Write the source-compatible JSON, then republish the theme tokens it owns so
    /// every writer (Settings view, MCP, future panels) applies the same state.
    fn persist(&self, settings: &Settings) -> Result<(), SettingsServiceError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(settings)?;
        std::fs::write(&self.path, serialized)?;
        photag_ui_bridge::theme::set_accent_name(&settings.accent_name);
        photag_ui_bridge::theme::set_region_depth_name(&settings.region_separation);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsServiceError {
    #[error("settings lock is poisoned")]
    Poisoned,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default)]
pub struct SettingsConfig {
    pub path: Option<PathBuf>,
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    type Config = SettingsConfig;
    type Input = SettingsConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = SettingsPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.settings".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let service = Arc::new(match &config.path {
            Some(path) => SettingsService::open_at(path.clone()),
            None => SettingsService::open_default(),
        });
        let settings = service
            .snapshot()
            .map_err(|error| SettingsPluginError::Publish(error.to_string()))?;
        let surfaces = context
            .try_service::<UiBridgeService>()
            .map_err(|error| SettingsPluginError::Publish(error.to_string()))?
            .surfaces
            .clone();
        let command_surfaces = Arc::clone(&surfaces);
        let _settings_command = context
            .on::<UiCommandInvoked, _>(observer_sync(move |_, command_id: String| {
                if command_id == "settings/toggle"
                    && let Err(error) = command_surfaces.toggle("settings")
                {
                    tracing::error!("could not toggle Settings surface: {error}");
                }
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| SettingsPluginError::EventRegistration(error.to_string()))?;
        let navigation = context
            .try_service::<photag_plugin_navigation::NavigationService>()
            .ok()
            .map(|service| Arc::downgrade(&service));
        let sidebar_settings = Arc::clone(&service);
        let last_sidebar_revision = std::sync::Mutex::new(0u64);
        let _sidebar_folders = context
            .on::<photag_contracts::SidebarFoldersChanged, _>(observer_sync(
                move |_, update: photag_contracts::SidebarFoldersUpdate| {
                    sidebar_settings.apply_sidebar_update(&last_sidebar_revision, update)
                },
            ))
            .map_err(|error| SettingsPluginError::EventRegistration(error.to_string()))?;
        if let Some(navigation) = navigation.as_ref().and_then(|weak| weak.upgrade()) {
            navigation
                .sidebar_folders()
                .restore(settings.sidebar_folders.clone());
        }
        let updater = context
            .try_service::<photag_plugin_updater::UpdaterService>()
            .ok();
        let session = context
            .try_service::<photag_plugin_session::SessionService>()
            .ok();
        photag_ui_bridge::theme::set_accent_name(&settings.accent_name);
        photag_ui_bridge::theme::set_region_depth_name(&settings.region_separation);
        let publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| SettingsPluginError::Publish(error.to_string()))?;
        context
            .emit::<TagInteractionPreferencesChanged>(
                Routing::Unscoped,
                interaction_preferences(&settings),
            )
            .await
            .map_err(|error| SettingsPluginError::Publish(error.to_string()))?;
        register_for_plugin(
            &context,
            "photag.settings",
            [
                UiContribution::new(
                    "settings/layout",
                    ContributionKind::Layout {
                        label: "Settings".into(),
                        order: 90,
                    },
                ),
                UiContribution::new(
                    "settings/slot",
                    ContributionKind::Slot {
                        label: "workspace.modal".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "settings/panel",
                    ContributionKind::Panel {
                        label: "Settings".into(),
                        slot: "workspace.modal".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "settings/view",
                    ContributionKind::Widget {
                        label: "PhoTag Settings".into(),
                        slot: "workspace.modal".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "settings/toggle",
                    ContributionKind::Command {
                        label: "Settings".into(),
                    },
                ),
                UiContribution::new(
                    "settings/menu-window",
                    ContributionKind::MenuItem {
                        label: "Settings…".into(),
                        menu: "Window".into(),
                        command_id: "settings/toggle".into(),
                        order: 0,
                    },
                ),
            ],
        )
        .map_err(|error| SettingsPluginError::UiRegistration(error.to_string()))?;
        register_views_for_plugin(
            &context,
            "photag.settings",
            [view::settings(
                service,
                navigation,
                updater,
                session,
                surfaces,
                context.clone(),
            )],
        )
        .map_err(|error| SettingsPluginError::UiRegistration(error.to_string()))?;
        drop(publication);
        Ok(())
    }
}

pub(crate) fn interaction_preferences(settings: &Settings) -> TagInteractionPreferences {
    TagInteractionPreferences {
        exclude_modifier: match settings.exclude_modifier {
            crate::ExcludeModifier::None => TagExcludeModifier::None,
            crate::ExcludeModifier::Ctrl => TagExcludeModifier::Ctrl,
            crate::ExcludeModifier::Shift => TagExcludeModifier::Shift,
            crate::ExcludeModifier::Alt => TagExcludeModifier::Alt,
        },
        search_match: match settings.tag_search_match {
            crate::TagSearchMatch::Contains => photag_contracts::TagSearchMatch::Contains,
            crate::TagSearchMatch::StartsWith => photag_contracts::TagSearchMatch::StartsWith,
        },
        search_persist: settings.tag_search_persist,
        merge_reparent_children: settings.tag_merge_children == crate::TagMergeChildren::Reparent,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsPluginError {
    #[error("settings service publication failed: {0}")]
    Publish(String),
    #[error("settings UI registration failed: {0}")]
    UiRegistration(String),
    #[error("settings command registration failed: {0}")]
    EventRegistration(String),
}

#[cfg(test)]
mod review_tests;
