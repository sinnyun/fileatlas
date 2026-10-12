//! Lifecycle-owned UI contribution metadata and GPUI view mounts for Cordis plugins.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use cordis::{Event, Service};

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
}

impl UiBridgeService {
    /// Construct the bridge around one catalog instance.
    pub fn new(catalog: Arc<UiCatalog>) -> Self {
        Self {
            catalog,
            views: Arc::new(UiViewCatalog::default()),
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
    Command { label: String },
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
            )
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

        fiber.dispose().await.unwrap();
        assert!(catalog.snapshot().unwrap().is_empty());
        assert!(bridge.views.snapshot().unwrap().is_empty());
    }
}
