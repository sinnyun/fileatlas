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

/// A plugin panel hosted in its own native window, never mounted in the workspace.
#[derive(Clone)]
pub struct NativePanel {
    pub id: String,
    pub definition: PanelDefinition,
    pub activation: u64,
}

#[derive(Default)]
struct State {
    definitions: BTreeMap<String, (String, PanelDefinition)>,
    instances: BTreeMap<String, (String, String)>,
    next: u64,
    activation: u64,
    native: BTreeMap<String, NativePanel>,
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

    /// Snapshot native instances for host window creation, activation and cleanup.
    pub fn native_panels(&self) -> Result<Vec<NativePanel>, String> {
        Ok(self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?
            .native
            .values()
            .cloned()
            .collect())
    }

    /// Single returns/focuses the existing instance; Multiple creates a fresh content view.
    pub fn open(self: &Arc<Self>, definition_id: &str) -> Result<String, String> {
        self.open_presented(definition_id, false)
    }

    /// Preserve single-instance semantics across workspace/native mode switches.
    pub fn open_presented(
        self: &Arc<Self>,
        definition_id: &str,
        native: bool,
    ) -> Result<String, String> {
        let replacement = {
            let state = self
                .state
                .lock()
                .map_err(|_| "Panel catalog lock poisoned")?;
            let single = state
                .definitions
                .get(definition_id)
                .is_some_and(|(_, d)| d.multiplicity == PanelMultiplicity::Single);
            state
                .instances
                .iter()
                .find(|(_, (_, def))| single && def == definition_id)
                .filter(|(id, _)| state.native.contains_key(*id) != native)
                .map(|(id, _)| id.clone())
        };
        if let Some(id) = replacement {
            self.close(&id)?;
        }
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
                let id = id.clone();
                state.activation += 1;
                let activation = state.activation;
                if let Some(panel) = state.native.get_mut(&id) {
                    panel.activation = activation;
                } else {
                    self.surfaces
                        .set_value("workspace.activate", &activation.to_string())?;
                }
                self.surfaces.set_value("panel.active", &id)?;
                return Ok(id);
            }
        }
        state.next += 1;
        let id = format!("{}@{}", definition_id, state.next);
        state.activation += 1;
        if native {
            let mut definition = definition;
            definition.options.id = id.clone();
            let activation = state.activation;
            state.native.insert(
                id.clone(),
                NativePanel {
                    id: id.clone(),
                    definition,
                    activation,
                },
            );
            self.surfaces.set_open(format!("panel.{id}"), true)?;
            self.surfaces.set_value("panel.active", &id)?;
            self.surfaces
                .set_value(&format!("panel.{id}.native"), "true")?;
            state
                .instances
                .insert(id.clone(), (owner, definition_id.into()));
            return Ok(id);
        }
        self.surfaces
            .set_value("workspace.activate", &state.activation.to_string())?;
        let mut options = definition.options;
        options.id = id.clone();
        self.surfaces.set_value(
            &format!("panel.{id}.modal"),
            if options.modal { "true" } else { "false" },
        )?;
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

    /// Close this tool definition without relying on the globally active panel.
    pub fn close_definition(&self, definition_id: &str) -> Result<(), String> {
        let ids = self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?
            .instances
            .iter()
            .filter(|(_, (_, def))| def == definition_id)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            self.close(&id)?;
        }
        Ok(())
    }

    pub fn close(&self, id: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Panel catalog lock poisoned")?;
        if state.instances.remove(id).is_none() {
            return Err(format!("Unknown panel instance {id}"));
        }
        state.native.remove(id);
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
            state.native.remove(&id);
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
    fn native_single_reactivates_switches_and_cleans_up() -> Result<(), String> {
        let c = catalog();
        c.register("tool", definition("tool", PanelMultiplicity::Single))?;
        let native = c.open_presented("tool", true)?;
        assert!(c.views.snapshot().map_err(|e| e.to_string())?.is_empty());
        let activation = c.native_panels()?[0].activation;
        assert_eq!(c.open_presented("tool", true)?, native);
        assert!(c.native_panels()?[0].activation > activation);
        let embedded = c.open("tool")?;
        assert_ne!(embedded, native);
        assert!(c.native_panels()?.is_empty());
        assert!(!c.surfaces.is_open(&format!("panel.{native}")));
        let again = c.open_presented("tool", true)?;
        assert_ne!(again, embedded);
        assert!(c.views.snapshot().map_err(|e| e.to_string())?.is_empty());
        c.unregister_owner("tool")?;
        assert!(c.native_panels()?.is_empty());
        assert!(!c.surfaces.any_panel_open());
        Ok(())
    }

    #[test]
    fn native_and_workspace_instances_do_not_override_each_other() -> Result<(), String> {
        let c = catalog();
        c.register("tool", definition("native", PanelMultiplicity::Single))?;
        c.register("other", definition("embedded", PanelMultiplicity::Single))?;
        let native = c.open_presented("native", true)?;
        let embedded = c.open("embedded")?;
        assert_eq!(c.native_panels()?.len(), 1);
        c.close(&embedded)?;
        assert_eq!(c.native_panels()?[0].id, native);
        c.close_definition("native")?;
        assert!(!c.surfaces.is_open(&format!("panel.{native}")));
        assert!(c.native_panels()?.is_empty());
        Ok(())
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
