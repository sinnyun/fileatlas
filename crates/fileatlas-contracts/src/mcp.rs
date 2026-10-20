//! Lifecycle-owned MCP tool contributions from product feature plugins.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, Weak},
};

use cordis::Service;
use serde_json::{Map, Value};

pub type McpToolFuture = Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'static>>;
pub type McpToolHandler = Arc<dyn Fn(Value) -> McpToolFuture + Send + Sync + 'static>;

#[derive(Clone)]
pub struct McpToolDefinition {
    pub name: String,
    pub title: String,
    pub description: String,
    pub input_schema: Map<String, Value>,
    pub mutating: bool,
    pub confirmation_required: bool,
    pub handler: McpToolHandler,
}

impl std::fmt::Debug for McpToolDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpToolDefinition")
            .field("name", &self.name)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("mutating", &self.mutating)
            .field("confirmation_required", &self.confirmation_required)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
pub struct McpToolRegistry {
    tools: Mutex<BTreeMap<String, (String, McpToolDefinition)>>,
}

impl Service for McpToolRegistry {
    const NAME: &'static str = "fileatlas/mcp-tool-registry";
}

impl McpToolRegistry {
    pub fn register(
        self: &Arc<Self>,
        owner: impl Into<String>,
        contributions: impl IntoIterator<Item = McpToolDefinition>,
    ) -> Result<Arc<McpToolRegistration>, McpRegistrationError> {
        let owner = owner.into();
        if owner.trim().is_empty() {
            return Err(McpRegistrationError::EmptyOwner);
        }
        let batch = contributions.into_iter().collect::<Vec<_>>();
        let mut names = std::collections::BTreeSet::new();
        for tool in &batch {
            if tool.name.trim().is_empty() || !tool.name.contains('.') {
                return Err(McpRegistrationError::InvalidName(tool.name.clone()));
            }
            if !tool
                .name
                .starts_with(&format!("{}.", owner.trim_start_matches("fileatlas.")))
            {
                return Err(McpRegistrationError::OwnerNamespaceMismatch(
                    tool.name.clone(),
                ));
            }
            if !names.insert(tool.name.clone()) {
                return Err(McpRegistrationError::DuplicateName(tool.name.clone()));
            }
            if tool.confirmation_required && !tool.mutating {
                return Err(McpRegistrationError::ConfirmationWithoutMutation(
                    tool.name.clone(),
                ));
            }
        }
        let mut tools = self
            .tools
            .lock()
            .map_err(|_| McpRegistrationError::Unavailable)?;
        if let Some(name) = batch
            .iter()
            .find_map(|tool| tools.contains_key(&tool.name).then(|| tool.name.clone()))
        {
            return Err(McpRegistrationError::DuplicateName(name));
        }
        for tool in batch {
            tools.insert(tool.name.clone(), (owner.clone(), tool));
        }
        Ok(Arc::new(McpToolRegistration {
            registry: Arc::downgrade(self),
            owner,
        }))
    }

    pub fn snapshot(&self) -> Result<Vec<McpToolDefinition>, McpRegistrationError> {
        self.tools
            .lock()
            .map(|tools| tools.values().map(|(_, tool)| tool.clone()).collect())
            .map_err(|_| McpRegistrationError::Unavailable)
    }

    fn unregister_owner(&self, owner: &str) {
        if let Ok(mut tools) = self.tools.lock() {
            tools.retain(|_, (tool_owner, _)| tool_owner != owner);
        }
    }
}

pub struct McpToolRegistration {
    registry: Weak<McpToolRegistry>,
    owner: String,
}

impl Drop for McpToolRegistration {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.unregister_owner(&self.owner);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpRegistrationError {
    #[error("MCP contribution owner is empty")]
    EmptyOwner,
    #[error("MCP tool name must be namespaced: {0}")]
    InvalidName(String),
    #[error("MCP tool name is outside its owner's namespace: {0}")]
    OwnerNamespaceMismatch(String),
    #[error("MCP tool name is already registered: {0}")]
    DuplicateName(String),
    #[error("non-mutating MCP tool cannot require mutation confirmation: {0}")]
    ConfirmationWithoutMutation(String),
    #[error("MCP tool registry is unavailable")]
    Unavailable,
}
