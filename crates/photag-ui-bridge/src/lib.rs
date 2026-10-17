//! Lifecycle-owned UI contribution metadata and GPUI view mounts for Cordis plugins.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, RwLock};

use cordis::{Event, Service};

pub mod i18n;
pub mod panel_catalog;
pub mod panels;
pub mod read_model;
pub mod refresh;
pub mod tag_capsule;
pub mod theme;
mod views;
pub mod winfx;
pub use views::{
    OwnedUiView, UiViewCatalog, UiViewContribution, UiViewFactory, register_views_for_plugin,
};

/// Cordis service contract for the application's lifecycle-managed UI catalog.
pub struct UiBridgeService {
    /// Shared catalog read by the GPUI application thread.
    pub catalog: Arc<UiCatalog>,
    /// Plugin-owned GPUI views mounted by the native shell.
    pub views: Arc<UiViewCatalog>,
    /// Shared open/closed state for lifecycle-owned floating UI surfaces.
    pub surfaces: Arc<UiSurfaceState>,
    pub panels: Arc<panel_catalog::PanelCatalog>,
}

#[derive(Debug, Default)]
pub struct UiSurfaceState {
    values: RwLock<BTreeMap<String, String>>,
    open: RwLock<HashSet<String>>,
    key_capture: RwLock<HashSet<String>>,
    suppressed_escape: RwLock<HashSet<String>>,
}

impl UiSurfaceState {
    /// Remove instance-local geometry and visibility when its owning view is disposed.
    pub fn remove_panel(&self, id: &str) -> Result<(), String> {
        self.set_open(format!("panel.{id}"), false)?;
        let next = self
            .open_panel_ids()
            .into_iter()
            .next_back()
            .unwrap_or_default();
        let mut values = self
            .values
            .write()
            .map_err(|_| "UI value lock is poisoned")?;
        let prefix = format!("panel.{id}.");
        values.retain(|key, _| !key.starts_with(&prefix));
        if values
            .get("panel.active")
            .is_some_and(|active| active == id)
        {
            values.insert("panel.active".into(), next);
        }
        Ok(())
    }
    pub fn any_panel_open(&self) -> bool {
        self.open
            .read()
            .is_ok_and(|open| open.iter().any(|id| id.starts_with("panel.")))
    }
    pub fn open_panel_ids(&self) -> Vec<String> {
        self.open
            .read()
            .map(|open| {
                let mut ids = open
                    .iter()
                    .filter_map(|id| id.strip_prefix("panel.").map(str::to_owned))
                    .collect::<Vec<_>>();
                ids.sort();
                ids
            })
            .unwrap_or_default()
    }
    pub fn value(&self, id: &str) -> Option<String> {
        self.values
            .read()
            .ok()
            .and_then(|values| values.get(id).cloned())
    }

    pub fn set_value(&self, id: &str, value: &str) -> Result<(), String> {
        self.values
            .write()
            .map_err(|_| "UI value lock is poisoned".to_owned())?
            .insert(id.to_owned(), value.to_owned());
        Ok(())
    }
    pub fn is_open(&self, id: &str) -> bool {
        self.open.read().is_ok_and(|open| open.contains(id))
    }

    pub fn set_open(&self, id: impl Into<String>, open: bool) -> Result<(), String> {
        let mut surfaces = self
            .open
            .write()
            .map_err(|_| "UI surface state lock is poisoned".to_owned())?;
        let id = id.into();
        if surfaces.contains(&id) != open {
            tracing::info!(surface=%id, open, "surface visibility changed");
        }
        if open {
            surfaces.insert(id);
        } else {
            surfaces.remove(&id);
        }
        Ok(())
    }

    pub fn toggle(&self, id: impl Into<String>) -> Result<bool, String> {
        let mut surfaces = self
            .open
            .write()
            .map_err(|_| "UI surface state lock is poisoned".to_owned())?;
        let id = id.into();
        if surfaces.remove(&id) {
            Ok(false)
        } else {
            surfaces.insert(id);
            Ok(true)
        }
    }

    pub fn is_capturing_key(&self, id: &str) -> bool {
        self.key_capture
            .read()
            .is_ok_and(|capturing| capturing.contains(id))
    }

    pub fn set_capturing_key(&self, id: impl Into<String>, active: bool) -> Result<(), String> {
        let mut capturing = self
            .key_capture
            .write()
            .map_err(|_| "UI key-capture state lock is poisoned".to_owned())?;
        let id = id.into();
        if active {
            capturing.insert(id);
        } else {
            capturing.remove(&id);
        }
        Ok(())
    }

    pub fn suppress_next_escape(&self, id: impl Into<String>) -> Result<(), String> {
        self.suppressed_escape
            .write()
            .map_err(|_| "UI Escape state lock is poisoned".to_owned())?
            .insert(id.into());
        Ok(())
    }

    pub fn take_suppressed_escape(&self, id: &str) -> bool {
        self.suppressed_escape
            .write()
            .is_ok_and(|mut suppressed| suppressed.remove(id))
    }
}

impl UiBridgeService {
    /// Construct the bridge around one catalog instance.
    pub fn new(catalog: Arc<UiCatalog>) -> Self {
        let views = Arc::new(UiViewCatalog::default());
        let surfaces = Arc::new(UiSurfaceState::default());
        Self {
            catalog,
            panels: Arc::new(panel_catalog::PanelCatalog::new(
                views.clone(),
                surfaces.clone(),
            )),
            views,
            surfaces,
        }
    }
}

impl Service for UiBridgeService {
    const NAME: &'static str = "photag/ui-bridge";
}

/// A plugin-owned contribution registered into a named UI surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiContribution {
    /// Stable, globally unique contribution identifier.
    pub id: String,
    /// Contribution role and host placement metadata.
    pub kind: ContributionKind,
}

impl UiContribution {
    /// Construct a contribution from a stable identifier and kind.
    pub fn new(id: impl Into<String>, kind: ContributionKind) -> Self {
        Self {
            id: id.into(),
            kind,
        }
    }
}

/// UI extension types supported by the host bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContributionKind {
    /// Named app layout region, such as a main area or toolbar.
    Layout { label: String, order: i32 },
    /// Dock surface that can host panels.
    Dock {
        label: String,
        side: DockSide,
        order: i32,
    },
    /// Panel mounted into a slot.
    Panel {
        label: String,
        slot: String,
        order: i32,
    },
    /// Named insertion point for widgets or panels.
    Slot { label: String, order: i32 },
    /// A presentational widget mounted in a slot.
    Widget {
        label: String,
        slot: String,
        order: i32,
    },
    /// Executable command exposed to menus and keybindings.
    Command { label: String, needs: CommandNeeds },
    /// Menu entry referring to a command identifier.
    MenuItem {
        label: String,
        menu: String,
        command_id: String,
        order: i32,
    },
    /// Key chord referring to a command identifier.
    Keybinding { key: String, command_id: String },
}

/// Preconditions used by the shared command catalog and MCP dispatcher.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CommandNeeds {
    /// No operation-specific precondition.
    #[default]
    None,
    /// Requires the host's explicit MCP mutation opt-in.
    MutationOptIn,
    /// Requires an available undo entry and the MCP mutation opt-in.
    UndoAvailable,
    /// Requires an available redo entry and the MCP mutation opt-in.
    RedoAvailable,
    /// Requires mutation opt-in and `confirm: true` on MCP invocation.
    ConfirmedMutation,
}

/// Menu placement metadata associated with a registered command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMenuItem {
    pub id: String,
    pub label: String,
    pub menu: String,
    pub order: i32,
}

/// Keybinding metadata associated with a registered command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandKeybinding {
    pub id: String,
    pub key: String,
}

/// Stable command metadata projected from lifecycle-owned UI contributions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredCommand {
    pub owner: String,
    pub id: String,
    pub label: String,
    pub needs: CommandNeeds,
    pub menu_items: Vec<CommandMenuItem>,
    pub keybindings: Vec<CommandKeybinding>,
}

/// App chrome locations supported by the panel host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSide {
    /// Left dock.
    Left,
    /// Right dock.
    Right,
    /// Bottom dock.
    Bottom,
}

/// A contribution with its lifecycle owner attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedContribution {
    /// Cordis plugin identifier used for stop-time cleanup.
    pub owner: String,
    /// The registered contribution.
    pub contribution: UiContribution,
}

/// Thread-safe registry shared between Cordis plugin tasks and GPUI rendering.
#[derive(Debug, Default)]
pub struct UiCatalog {
    contributions: RwLock<BTreeMap<String, OwnedContribution>>,
}

/// Errors returned when a plugin registers an invalid contribution set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrationError {
    /// Owner identifiers must not be empty.
    EmptyOwner,
    /// Contribution identifiers must not be empty.
    EmptyId,
    /// The identifier already belongs to another contribution.
    DuplicateId(String),
    /// The catalog lock was poisoned by a panicking thread.
    LockPoisoned,
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyOwner => formatter.write_str("UI contribution owner is empty"),
            Self::EmptyId => formatter.write_str("UI contribution id is empty"),
            Self::DuplicateId(id) => write!(formatter, "UI contribution id `{id}` already exists"),
            Self::LockPoisoned => formatter.write_str("UI catalog lock is poisoned"),
        }
    }
}

impl std::error::Error for RegistrationError {}

/// Register contributions and bind their removal to the current Cordis
/// plugin generation. Disposing or replacing the plugin removes its UI entries.
pub fn register_for_plugin(
    context: &cordis::Context,
    owner: impl Into<String>,
    contributions: impl IntoIterator<Item = UiContribution>,
) -> Result<(), PluginRegistrationError> {
    let owner = owner.into();
    let bridge = context
        .try_service::<UiBridgeService>()
        .map_err(PluginRegistrationError::Service)?;
    let catalog = Arc::clone(&bridge.catalog);
    let cleanup_catalog = Arc::clone(&catalog);
    let cleanup_owner = owner.clone();
    context
        .effect_sync(move || {
            let _ = cleanup_catalog.unregister_owner(&cleanup_owner);
        })
        .map_err(PluginRegistrationError::Lifecycle)?;
    catalog
        .register(owner, contributions)
        .map_err(PluginRegistrationError::Catalog)
}

/// A failure while attaching UI contributions to a live Cordis plugin.
#[derive(Debug)]
pub enum PluginRegistrationError {
    /// The application has not published its bridge service.
    Service(cordis::service::ServiceLookupError),
    /// Cordis did not admit the cleanup obligation.
    Lifecycle(cordis::effect::EffectRegistrationError),
    /// The catalog rejected the contribution batch.
    Catalog(RegistrationError),
}

impl std::fmt::Display for PluginRegistrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Service(error) => write!(formatter, "UI bridge service unavailable: {error}"),
            Self::Lifecycle(error) => write!(
                formatter,
                "could not bind UI cleanup to plugin lifecycle: {error}"
            ),
            Self::Catalog(error) => write!(formatter, "UI contributions rejected: {error}"),
        }
    }
}

impl std::error::Error for PluginRegistrationError {}

impl UiCatalog {
    /// Atomically register contributions under one Cordis plugin owner.
    ///
    /// The whole batch is rejected if any identifier is invalid or already
    /// present; no partial registrations are left behind.
    pub fn register(
        &self,
        owner: impl Into<String>,
        contributions: impl IntoIterator<Item = UiContribution>,
    ) -> Result<(), RegistrationError> {
        let owner = owner.into();
        if owner.trim().is_empty() {
            return Err(RegistrationError::EmptyOwner);
        }
        let batch = contributions.into_iter().collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        for contribution in &batch {
            if contribution.id.trim().is_empty() {
                return Err(RegistrationError::EmptyId);
            }
            if !seen.insert(contribution.id.clone()) {
                return Err(RegistrationError::DuplicateId(contribution.id.clone()));
            }
        }

        let mut registered = self
            .contributions
            .write()
            .map_err(|_| RegistrationError::LockPoisoned)?;
        if let Some(existing) = batch
            .iter()
            .find(|contribution| registered.contains_key(&contribution.id))
        {
            return Err(RegistrationError::DuplicateId(existing.id.clone()));
        }
        for contribution in batch {
            registered.insert(
                contribution.id.clone(),
                OwnedContribution {
                    owner: owner.clone(),
                    contribution,
                },
            );
        }
        Ok(())
    }

    /// Remove every UI resource owned by one stopped Cordis plugin.
    pub fn unregister_owner(&self, owner: &str) -> Result<usize, RegistrationError> {
        let mut registered = self
            .contributions
            .write()
            .map_err(|_| RegistrationError::LockPoisoned)?;
        let before = registered.len();
        registered.retain(|_, contribution| contribution.owner != owner);
        Ok(before - registered.len())
    }

    /// Return a stable, identifier-sorted snapshot for the GPUI thread.
    pub fn snapshot(&self) -> Result<Vec<OwnedContribution>, RegistrationError> {
        self.contributions
            .read()
            .map(|registered| registered.values().cloned().collect())
            .map_err(|_| RegistrationError::LockPoisoned)
    }

    /// Return commands with their related menu items and keybindings. This is
    /// the common metadata source used by the Shell UI and MCP command tools.
    pub fn commands(&self) -> Result<Vec<RegisteredCommand>, RegistrationError> {
        let registered = self
            .contributions
            .read()
            .map_err(|_| RegistrationError::LockPoisoned)?;
        let mut commands = BTreeMap::<String, RegisteredCommand>::new();
        for owned in registered.values() {
            if let ContributionKind::Command { label, needs } = &owned.contribution.kind {
                commands.insert(
                    owned.contribution.id.clone(),
                    RegisteredCommand {
                        owner: owned.owner.clone(),
                        id: owned.contribution.id.clone(),
                        label: label.clone(),
                        needs: *needs,
                        menu_items: Vec::new(),
                        keybindings: Vec::new(),
                    },
                );
            }
        }
        for owned in registered.values() {
            match &owned.contribution.kind {
                ContributionKind::MenuItem {
                    label,
                    menu,
                    command_id,
                    order,
                } => {
                    if let Some(command) = commands.get_mut(command_id) {
                        command.menu_items.push(CommandMenuItem {
                            id: owned.contribution.id.clone(),
                            label: label.clone(),
                            menu: menu.clone(),
                            order: *order,
                        });
                    }
                }
                ContributionKind::Keybinding { key, command_id } => {
                    if let Some(command) = commands.get_mut(command_id) {
                        command.keybindings.push(CommandKeybinding {
                            id: owned.contribution.id.clone(),
                            key: key.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
        for command in commands.values_mut() {
            command
                .menu_items
                .sort_by_key(|item| (item.order, item.id.clone()));
            command
                .keybindings
                .sort_by(|left, right| left.id.cmp(&right.id));
        }
        Ok(commands.into_values().collect())
    }
}

/// Typed event raised by the GPUI host when a contributed command is invoked.
pub struct UiCommandInvoked;

impl Event for UiCommandInvoked {
    const NAME: &'static str = "photag/ui/command-invoked";
    type Args = String;
    type Output = ();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::sync::Arc;

    use cordis::{Context, Plugin, PreparedPlugin};
    use gpui_kit::{AnyView, App, AppContext as _, ParentElement as _, Window};

    fn widget(id: &str) -> UiContribution {
        UiContribution::new(
            id,
            ContributionKind::Widget {
                label: id.into(),
                slot: "main".into(),
                order: 0,
            },
        )
    }

    #[test]
    fn batch_registration_is_atomic_on_duplicate() {
        let catalog = UiCatalog::default();
        catalog
            .register("library", [widget("library/grid")])
            .unwrap();
        let result = catalog.register("search", [widget("search/toolbar"), widget("library/grid")]);
        assert_eq!(
            result,
            Err(RegistrationError::DuplicateId("library/grid".into()))
        );
        let snapshot = catalog.snapshot().unwrap();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].owner, "library");
    }

    #[test]
    fn owner_cleanup_removes_only_that_plugins_resources() {
        let catalog = UiCatalog::default();
        catalog
            .register("library", [widget("library/grid")])
            .unwrap();
        catalog.register("tags", [widget("tags/tree")]).unwrap();
        assert_eq!(catalog.unregister_owner("library").unwrap(), 1);
        let snapshot = catalog.snapshot().unwrap();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].contribution.id, "tags/tree");
    }

    #[test]
    fn command_registry_joins_command_policy_menu_items_and_keybindings() {
        let catalog = UiCatalog::default();
        catalog
            .register(
                "history",
                [
                    UiContribution::new(
                        "history/undo",
                        ContributionKind::Command {
                            label: "Undo".into(),
                            needs: CommandNeeds::UndoAvailable,
                        },
                    ),
                    UiContribution::new(
                        "history/undo-menu",
                        ContributionKind::MenuItem {
                            label: "Undo last action".into(),
                            menu: "Edit".into(),
                            command_id: "history/undo".into(),
                            order: 10,
                        },
                    ),
                    UiContribution::new(
                        "history/undo-key",
                        ContributionKind::Keybinding {
                            key: "Ctrl+Z".into(),
                            command_id: "history/undo".into(),
                        },
                    ),
                ],
            )
            .unwrap();

        let commands = catalog.commands().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].owner, "history");
        assert_eq!(commands[0].id, "history/undo");
        assert_eq!(commands[0].needs, CommandNeeds::UndoAvailable);
        assert_eq!(commands[0].menu_items[0].menu, "Edit");
        assert_eq!(commands[0].menu_items[0].order, 10);
        assert_eq!(commands[0].keybindings[0].key, "Ctrl+Z");
    }

    struct RegisteringFeature;

    impl Plugin for RegisteringFeature {
        type Config = Vec<UiContribution>;
        type Input = Vec<UiContribution>;
        type PrepareError = Infallible;
        type ApplyError = PluginRegistrationError;

        fn prepare(&self, contributions: Self::Config) -> Result<Self::Input, Self::PrepareError> {
            Ok(contributions)
        }

        async fn apply(
            &self,
            context: Context,
            contributions: &Self::Input,
        ) -> Result<(), Self::ApplyError> {
            register_for_plugin(&context, "feature.test", contributions.clone())?;
            register_views_for_plugin(
                &context,
                "feature.test",
                [UiViewContribution::new(
                    "feature.test/view",
                    "main",
                    0,
                    test_view,
                )],
            )?;
            panel_catalog::register_panel_for_plugin(
                &context,
                "feature.test",
                panel_catalog::PanelDefinition {
                    options: panels::PanelOptions::new(
                        "feature.test/panel",
                        "Test panel",
                        (500., 400.),
                    ),
                    multiplicity: panel_catalog::PanelMultiplicity::Multiple,
                    factory: Arc::new(test_view),
                },
            )
            .unwrap();
            Ok(())
        }
    }

    struct TestView;

    impl gpui_kit::Render for TestView {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            gpui_kit::div().child("test view")
        }
    }

    fn test_view(_: &mut Window, app: &mut App) -> AnyView {
        app.new(|_| TestView).into()
    }

    #[tokio::test]
    async fn cordis_disposal_removes_feature_ui_contributions() {
        let context = Context::new();
        let catalog = Arc::new(UiCatalog::default());
        let bridge = Arc::new(UiBridgeService::new(Arc::clone(&catalog)));
        let _bridge_publication = context.provide(Arc::clone(&bridge)).unwrap();

        let plugin = RegisteringFeature;
        let input = plugin.prepare(vec![widget("feature.test/grid")]).unwrap();
        let fiber = context
            .spawn(PreparedPlugin::from_input(plugin, input))
            .await
            .unwrap();
        fiber.ready().await.unwrap();
        assert_eq!(catalog.snapshot().unwrap().len(), 1);
        assert_eq!(
            bridge.views.snapshot().unwrap().len(),
            1,
            "plugin registers an executable GPUI mount"
        );

        bridge.panels.open("feature.test/panel").unwrap();
        bridge.panels.open("feature.test/panel").unwrap();
        assert_eq!(bridge.views.snapshot().unwrap().len(), 3);
        fiber.dispose().await.unwrap();
        assert!(!bridge.surfaces.any_panel_open());
        assert!(bridge.panels.open("feature.test/panel").is_err());
        assert!(catalog.snapshot().unwrap().is_empty());
        assert!(bridge.views.snapshot().unwrap().is_empty());
    }
}
