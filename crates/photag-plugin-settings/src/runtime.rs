//! Runtime controls for optional feature plugins. Lifecycle work remains delegated
//! to Cordis FiberHandle disposal and Context::spawn; this type only supplies the
//! host's registered start recipes to the Settings surface.
use cordis::FiberHandle;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type PluginStarter = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<FiberHandle, String>> + Send>> + Send + Sync,
>;

struct RuntimeEntry {
    title: String,
    description: String,
    launch_modes: Vec<crate::PluginLaunchMode>,
    starter: PluginStarter,
    fiber: Option<FiberHandle>,
    busy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRuntimeStatus {
    pub id: String,
    pub title: String,
    pub description: String,
    pub launch_modes: Vec<crate::PluginLaunchMode>,
    pub enabled: bool,
    pub busy: bool,
}

#[derive(Default)]
pub struct PluginRuntimeRegistry {
    entries: Mutex<BTreeMap<String, RuntimeEntry>>,
}

impl PluginRuntimeRegistry {
    pub async fn register(
        &self,
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        launch_modes: Vec<crate::PluginLaunchMode>,
        fiber: Option<FiberHandle>,
        starter: PluginStarter,
    ) -> Result<(), String> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err("plugin ID must not be empty".into());
        }
        let mut entries = self.entries.lock().await;
        if entries.contains_key(&id) {
            return Err(format!("plugin {id} already registered"));
        }
        entries.insert(
            id,
            RuntimeEntry {
                title: title.into(),
                description: description.into(),
                launch_modes,
                starter,
                fiber,
                busy: false,
            },
        );
        Ok(())
    }

    pub async fn snapshot(&self) -> Vec<PluginRuntimeStatus> {
        self.entries
            .lock()
            .await
            .iter()
            .map(|(id, entry)| PluginRuntimeStatus {
                id: id.clone(),
                title: entry.title.clone(),
                description: entry.description.clone(),
                launch_modes: entry.launch_modes.clone(),
                enabled: entry.fiber.is_some(),
                busy: entry.busy,
            })
            .collect()
    }

    pub fn snapshot_now(&self) -> Vec<PluginRuntimeStatus> {
        self.entries
            .try_lock()
            .map(|entries| {
                entries
                    .iter()
                    .map(|(id, entry)| PluginRuntimeStatus {
                        id: id.clone(),
                        title: entry.title.clone(),
                        description: entry.description.clone(),
                        launch_modes: entry.launch_modes.clone(),
                        enabled: entry.fiber.is_some(),
                        busy: entry.busy,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), String> {
        let (starter, fiber) = {
            let mut entries = self.entries.lock().await;
            let entry = entries
                .get_mut(id)
                .ok_or_else(|| format!("plugin {id} is not runtime controllable"))?;
            if entry.busy {
                return Err(format!("plugin {} is changing state", entry.title));
            }
            if entry.fiber.is_some() == enabled {
                return Ok(());
            }
            entry.busy = true;
            (Arc::clone(&entry.starter), entry.fiber.take())
        };

        let (result, next_fiber) = if enabled {
            match starter().await {
                Ok(fiber) => (Ok(()), Some(fiber)),
                Err(error) => (Err(error), None),
            }
        } else if let Some(fiber) = fiber {
            match fiber.dispose().await {
                Ok(()) => (Ok(()), None),
                Err(error) => (Err(error.to_string()), Some(fiber)),
            }
        } else {
            (Err(format!("plugin {id} is already stopped")), None)
        };

        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(id) {
            entry.fiber = next_fiber;
            entry.busy = false;
        }
        result
    }
}
