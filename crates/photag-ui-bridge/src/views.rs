//! Executable GPUI view contributions and their Cordis lifecycle ownership.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use cordis::Context;
use gpui_kit::{AnyView, App, Window};

use crate::{PluginRegistrationError, RegistrationError, UiBridgeService};

/// Factory invoked on the GPUI thread to construct a plugin-owned view entity.
pub type UiViewFactory = Arc<dyn Fn(&mut Window, &mut App) -> AnyView + Send + Sync>;

/// Executable view mounted into a named UI slot.
#[derive(Clone)]
pub struct UiViewContribution {
    pub id: String,
    pub slot: String,
    pub order: i32,
    pub factory: UiViewFactory,
}

impl UiViewContribution {
    pub fn new(
        id: impl Into<String>,
        slot: impl Into<String>,
        order: i32,
        factory: impl Fn(&mut Window, &mut App) -> AnyView + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            slot: slot.into(),
            order,
            factory: Arc::new(factory),
        }
    }
}

impl std::fmt::Debug for UiViewContribution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UiViewContribution")
            .field("id", &self.id)
            .field("slot", &self.slot)
            .field("order", &self.order)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct OwnedUiView {
    pub owner: String,
    pub contribution: UiViewContribution,
}

#[derive(Debug, Default)]
pub struct UiViewCatalog {
    views: RwLock<BTreeMap<String, OwnedUiView>>,
}

impl UiViewCatalog {
    pub fn register(
        &self,
        owner: impl Into<String>,
        views: impl IntoIterator<Item = UiViewContribution>,
    ) -> Result<(), RegistrationError> {
        let owner = owner.into();
        if owner.trim().is_empty() {
            return Err(RegistrationError::EmptyOwner);
        }
        let batch = views.into_iter().collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        for view in &batch {
            if view.id.trim().is_empty() {
                return Err(RegistrationError::EmptyId);
            }
            if !seen.insert(view.id.clone()) {
                return Err(RegistrationError::DuplicateId(view.id.clone()));
            }
        }
        let mut registered = self
            .views
            .write()
            .map_err(|_| RegistrationError::LockPoisoned)?;
        if let Some(view) = batch.iter().find(|view| registered.contains_key(&view.id)) {
            return Err(RegistrationError::DuplicateId(view.id.clone()));
        }
        for contribution in batch {
            registered.insert(
                contribution.id.clone(),
                OwnedUiView {
                    owner: owner.clone(),
                    contribution,
                },
            );
        }
        Ok(())
    }

    pub fn unregister_owner(&self, owner: &str) -> Result<usize, RegistrationError> {
        let mut registered = self
            .views
            .write()
            .map_err(|_| RegistrationError::LockPoisoned)?;
        let before = registered.len();
        registered.retain(|_, view| view.owner != owner);
        Ok(before - registered.len())
    }

    pub fn snapshot(&self) -> Result<Vec<OwnedUiView>, RegistrationError> {
        let mut views: Vec<OwnedUiView> = self
            .views
            .read()
            .map(|views| views.values().cloned().collect())
            .map_err(|_| RegistrationError::LockPoisoned)?;
        views.sort_by(|left, right| {
            left.contribution
                .slot
                .cmp(&right.contribution.slot)
                .then(left.contribution.order.cmp(&right.contribution.order))
                .then(left.contribution.id.cmp(&right.contribution.id))
        });
        Ok(views)
    }
}

/// Register executable GPUI views and remove them with their Cordis plugin.
pub fn register_views_for_plugin(
    context: &Context,
    owner: impl Into<String>,
    views: impl IntoIterator<Item = UiViewContribution>,
) -> Result<(), PluginRegistrationError> {
    let owner = owner.into();
    let bridge = context
        .try_service::<UiBridgeService>()
        .map_err(PluginRegistrationError::Service)?;
    let catalog = Arc::clone(&bridge.views);
    let cleanup_catalog = Arc::clone(&catalog);
    let cleanup_owner = owner.clone();
    context
        .effect_sync(move || {
            let _ = cleanup_catalog.unregister_owner(&cleanup_owner);
        })
        .map_err(PluginRegistrationError::Lifecycle)?;
    catalog
        .register(owner, views)
        .map_err(PluginRegistrationError::Catalog)
}
