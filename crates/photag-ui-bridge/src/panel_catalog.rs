//! Cordis-owned panel definitions and on-demand single/multiple instances.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::panels::{self, PanelOptions};
use crate::{UiBridgeService, UiSurfaceState, UiViewCatalog, UiViewContribution, UiViewFactory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelMultiplicity {
    Single,
    Multiple,
}

#[derive(Clone)]
pub struct PanelDefinition {
    pub options: PanelOptions,
    pub multiplicity: PanelMultiplicity,
    pub factory: UiViewFactory,
}

#[derive(Default)]
struct State {
    definitions: BTreeMap<String, (String, PanelDefinition)>,
    instances: BTreeMap<String, (String, String)>,
    next: u64,
}

/// Public panel contract available as `UiBridgeService.panels`.
pub struct PanelCatalog {
    state: Mutex<State>,
    views: Arc<UiViewCatalog>,
    surfaces: Arc<UiSurfaceState>,
}

impl PanelCatalog {
    pub(crate) fn new(views: Arc<UiViewCatalog>, surfaces: Arc<UiSurfaceState>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            views,
            surfaces,
        }
    }

    pub fn register(&self, owner: &str, definition: PanelDefinition) -> Result<(), String> {
        if owner.trim().is_empty() || definition.options.id.trim().is_empty() {
            return Err("Panel owner and ID must be nonempty".into());
        }
        let o = &definition.options;
        if [
            o.default_size.0,
            o.default_size.1,
            o.min_size.0,
            o.min_size.1,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("Panel sizes must be finite and positive".into());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?;
        if state.definitions.contains_key(&o.id) {
            return Err(format!("Panel {} already registered", o.id));
        }
        state
            .definitions
            .insert(o.id.clone(), (owner.into(), definition));
        Ok(())
    }

    /// Single returns/focuses the existing instance; Multiple creates a fresh content view.
    pub fn open(self: &Arc<Self>, definition_id: &str) -> Result<String, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?;
        let (owner, definition) = state
            .definitions
            .get(definition_id)
            .cloned()
            .ok_or_else(|| format!("Unknown panel {definition_id}"))?;
        if definition.multiplicity == PanelMultiplicity::Single {
            if let Some((id, _)) = state
                .instances
                .iter()
                .find(|(_, (_, def))| def == definition_id)
            {
                self.surfaces.set_value("panel.active", id)?;
                return Ok(id.clone());
            }
        }
        state.next += 1;
        let id = format!("{}@{}", definition_id, state.next);
        let mut options = definition.options;
        options.id = id.clone();
        let visible = self.surfaces.clone();
        let surfaces = self.surfaces.clone();
        let visible_id = id.clone();
        let close_id = id.clone();
        let weak = Arc::downgrade(self);
        self.views
            .register(
                owner.clone(),
                [UiViewContribution::new(
                    format!("panel/{id}"),
                    "workspace.modal",
                    20,
                    move |window, app| {
                        let content = (definition.factory)(window, app);
                        let visible = visible.clone();
                        let visible_id = visible_id.clone();
                        let weak = weak.clone();
                        let close_id = close_id.clone();
                        panels::mount(
                            content,
                            options.clone(),
                            surfaces.clone(),
                            move || visible.is_open(&format!("panel.{visible_id}")),
                            move || {
                                if let Some(catalog) = weak.upgrade() {
                                    let _ = catalog.close(&close_id);
                                }
                            },
                            app,
                        )
                    },
                )
                .with_panel_id(id.clone())],
            )
            .map_err(|e| e.to_string())?;
        self.surfaces.set_open(format!("panel.{id}"), true)?;
        self.surfaces.set_value("panel.active", &id)?;
        state
            .instances
            .insert(id.clone(), (owner, definition_id.into()));
        Ok(id)
    }

    pub fn close(&self, id: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?;
        if state.instances.remove(id).is_none() {
            return Err(format!("Unknown panel instance {id}"));
        }
        self.views
            .unregister_id(&format!("panel/{id}"))
            .map_err(|e| e.to_string())?;
        self.surfaces.remove_panel(id)
    }

    pub fn unregister_owner(&self, owner: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?;
        state
            .definitions
            .retain(|_, (registered, _)| registered != owner);
        let ids = state
            .instances
            .iter()
            .filter(|(_, (registered, _))| registered == owner)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            state.instances.remove(&id);
            self.views
                .unregister_id(&format!("panel/{id}"))
                .map_err(|e| e.to_string())?;
            self.surfaces.remove_panel(&id)?;
        }
        Ok(())
    }
}

/// Register a reusable definition and bind all its instances to this plugin generation.
pub fn register_panel_for_plugin(
    context: &cordis::Context,
    owner: &str,
    definition: PanelDefinition,
) -> Result<(), String> {
    let bridge = context
        .try_service::<UiBridgeService>()
        .map_err(|e| e.to_string())?;
    let panels = bridge.panels.clone();
    let cleanup = panels.clone();
    let cleanup_owner = owner.to_owned();
    context
        .effect_sync(move || {
            let _ = cleanup.unregister_owner(&cleanup_owner);
        })
        .map_err(|e| e.to_string())?;
    panels.register(owner, definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> Arc<PanelCatalog> {
        Arc::new(PanelCatalog::new(Arc::default(), Arc::default()))
    }
    fn definition(id: &str, multiplicity: PanelMultiplicity) -> PanelDefinition {
        PanelDefinition {
            options: PanelOptions::new(id, id, (500., 400.)),
            multiplicity,
            factory: Arc::new(|_, _| panic!("Factory is only executed on GPUI thread")),
        }
    }
    #[test]
    fn single_reuses_multiple_is_independent_and_owner_cleanup_is_scoped() {
        let c = catalog();
        c.register("one", definition("single", PanelMultiplicity::Single))
            .unwrap();
        c.register("two", definition("multi", PanelMultiplicity::Multiple))
            .unwrap();
        let single = c.open("single").unwrap();
        assert_eq!(single, c.open("single").unwrap());
        let a = c.open("multi").unwrap();
        let b = c.open("multi").unwrap();
        assert_ne!(a, b);
        c.surfaces
            .set_value(&format!("panel.{a}.bounds"), "0,0,500,400")
            .unwrap();
        c.close(&a).unwrap();
        assert!(c.surfaces.value(&format!("panel.{a}.bounds")).is_none());
        assert!(c.surfaces.is_open(&format!("panel.{b}")));
        c.unregister_owner("one").unwrap();
        assert!(c.open("single").is_err());
        assert_eq!(c.views.snapshot().unwrap().len(), 1);
        c.unregister_owner("two").unwrap();
        assert!(!c.surfaces.any_panel_open());
        assert!(c.views.snapshot().unwrap().is_empty());
    }
}
