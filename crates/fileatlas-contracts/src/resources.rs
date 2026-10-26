//! Stable links that let independent feature plugins share media resources.
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Image,
    Video,
    Audio,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceLink {
    /// Provider-owned, durable URI (for example `file:///...` or a provider URI).
    pub uri: String,
    pub provider: String,
    pub kind: ResourceKind,
    pub title: String,
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedResource {
    pub link: ResourceLink,
    /// Local source/cache file, when the provider can expose one.
    pub local_path: Option<PathBuf>,
}

/// A Cordis feature plugin can publish links and resolve them through the shared catalog.
/// A provider remains the owner of its source, metadata, and cache lifecycle.
pub trait ResourceProvider: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn accepts(&self, uri: &str) -> bool;
    fn create_link(&self, source: &str) -> Result<ResourceLink, String>;
    fn resolve(&self, uri: &str) -> Result<ResolvedResource, String>;
    /// Open or play this resource in provider-owned UI, when supported.
    fn open(&self, _uri: &str) -> Result<bool, String> {
        Ok(false)
    }
}
