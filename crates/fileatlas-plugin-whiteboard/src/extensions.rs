use fileatlas_contracts::canvas::CanvasToolContribution;
use fileatlas_ui_bridge::refresh::Wake;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

pub type CanvasCommandDispatch = dyn Fn(String) + Send + Sync;
type Owned = (String, CanvasToolContribution);
struct State {
    tools: Mutex<BTreeMap<String, Owned>>,
    wake: Wake,
    dispatch: Arc<CanvasCommandDispatch>,
}

/// Cordis-published registry for lifecycle-owned canvas toolbar commands.
pub struct CanvasExtensionService {
    state: Arc<State>,
}
impl cordis::Service for CanvasExtensionService {
    const NAME: &'static str = "fileatlas/canvas-extensions";
}
impl CanvasExtensionService {
    pub fn new(dispatch: Arc<CanvasCommandDispatch>) -> Self {
        Self {
            state: Arc::new(State {
                tools: Mutex::new(BTreeMap::new()),
                wake: Wake::default(),
                dispatch,
            }),
        }
    }

    pub fn register_tool(
        &self,
        owner: &str,
        contribution: CanvasToolContribution,
    ) -> Result<CanvasToolRegistration, String> {
        if owner.trim().is_empty()
            || contribution.id.trim().is_empty()
            || contribution.command_id.trim().is_empty()
            || contribution.label.trim().is_empty()
        {
            return Err("canvas tool owner, ID, command and label are required".into());
        }
        let mut tools = self.state.tools.lock().map_err(|e| e.to_string())?;
        if tools.contains_key(&contribution.id) {
            return Err(format!(
                "canvas tool already registered: {}",
                contribution.id
            ));
        }
        let id = contribution.id.clone();
        tools.insert(id.clone(), (owner.to_owned(), contribution));
        drop(tools);
        self.state.wake.trigger();
        Ok(CanvasToolRegistration {
            state: Arc::downgrade(&self.state),
            id,
        })
    }

    pub fn tools(&self) -> Vec<CanvasToolContribution> {
        let mut tools = self
            .state
            .tools
            .lock()
            .map(|tools| {
                tools
                    .values()
                    .map(|(_, tool)| tool.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        tools.sort_by(|left, right| {
            (left.group, left.order, &left.id).cmp(&(right.group, right.order, &right.id))
        });
        tools
    }

    pub fn invoke(&self, id: &str) -> Result<(), String> {
        let command = self
            .state
            .tools
            .lock()
            .map_err(|e| e.to_string())?
            .get(id)
            .map(|(_, tool)| tool.command_id.clone())
            .ok_or_else(|| format!("canvas tool is no longer registered: {id}"))?;
        (self.state.dispatch)(command);
        Ok(())
    }

    pub fn wake(&self) -> Wake {
        self.state.wake.clone()
    }
}

/// Keep this token in the contributing plugin's Cordis effect; dropping it removes
/// that plugin's toolbar item and wakes the canvas view to repaint.
pub struct CanvasToolRegistration {
    state: Weak<State>,
    id: String,
}
impl Drop for CanvasToolRegistration {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            if let Ok(mut tools) = state.tools.lock() {
                if tools.remove(&self.id).is_some() {
                    state.wake.trigger();
                }
            }
        }
    }
}

/// Register a toolbar command under a Cordis owner and bind its token to that
/// owner's stop lifecycle automatically.
pub fn register_tool_for_plugin(
    context: &cordis::Context,
    owner: &str,
    contribution: CanvasToolContribution,
) -> Result<(), String> {
    let service = context
        .try_service::<CanvasExtensionService>()
        .map_err(|error| error.to_string())?;
    let registration = service.register_tool(owner, contribution)?;
    context
        .effect_sync(move || drop(registration))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fileatlas_contracts::canvas::CanvasToolGroup;

    #[test]
    fn toolbar_commands_sort_dispatch_and_remove_with_their_registration() -> Result<(), String> {
        let called = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&called);
        let registry = CanvasExtensionService::new(Arc::new(move |command| {
            if let Ok(mut called) = sink.lock() {
                called.push(command);
            }
        }));
        let later = registry.register_tool(
            "feature.b",
            CanvasToolContribution::new("b/tool", "b/run", "B", CanvasToolGroup::Extensions, 20),
        )?;
        let earlier = registry.register_tool(
            "feature.a",
            CanvasToolContribution::new("a/tool", "a/run", "A", CanvasToolGroup::Extensions, 10),
        )?;

        let tools = registry.tools();
        assert_eq!(tools.first().map(|tool| tool.id.as_str()), Some("a/tool"));
        assert!(
            registry
                .register_tool(
                    "feature.c",
                    CanvasToolContribution::new(
                        "a/tool",
                        "c/run",
                        "Duplicate",
                        CanvasToolGroup::Extensions,
                        30
                    ),
                )
                .is_err()
        );
        registry.invoke("b/tool")?;
        assert_eq!(
            *called.lock().map_err(|error| error.to_string())?,
            ["b/run"]
        );

        drop(earlier);
        drop(later);
        assert!(registry.tools().is_empty());
        assert!(registry.invoke("b/tool").is_err());
        Ok(())
    }
}
