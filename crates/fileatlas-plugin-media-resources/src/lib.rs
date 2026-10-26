//! Shared resource links and provider registry for independent feature plugins.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
use cordis::{Context, Plugin, Service};
use fileatlas_contracts::resources::{
    ResolvedResource, ResourceKind, ResourceLink, ResourceProvider,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, Weak},
};

type Providers = BTreeMap<String, Arc<dyn ResourceProvider>>;
struct CatalogInner(RwLock<Providers>);

/// Shared catalog. The catalog normalizes access while providers own data and metadata.
pub struct MediaResourceService {
    inner: Arc<CatalogInner>,
}
impl Service for MediaResourceService {
    const NAME: &'static str = "fileatlas/media-resources";
}
impl MediaResourceService {
    fn new() -> Self {
        let inner = Arc::new(CatalogInner(RwLock::new(BTreeMap::new())));
        if let Ok(mut providers) = inner.0.write() {
            providers.insert("local-file".into(), Arc::new(LocalFileProvider));
        }
        Self { inner }
    }

    pub fn register_provider(
        &self,
        provider: Arc<dyn ResourceProvider>,
    ) -> Result<ResourceProviderRegistration, String> {
        let id = provider.id().to_owned();
        let mut providers = self.inner.0.write().map_err(|e| e.to_string())?;
        if providers.contains_key(&id) {
            return Err(format!("resource provider already registered: {id}"));
        }
        providers.insert(id.clone(), provider);
        Ok(ResourceProviderRegistration {
            inner: Arc::downgrade(&self.inner),
            id,
        })
    }

    pub fn link_file(&self, path: impl AsRef<Path>) -> Result<ResourceLink, String> {
        self.create_link("local-file", &path.as_ref().to_string_lossy())
    }
    pub fn create_link(&self, provider_id: &str, source: &str) -> Result<ResourceLink, String> {
        self.provider(provider_id)?.create_link(source)
    }
    pub fn local_path(&self, uri: &str) -> Result<PathBuf, String> {
        self.resolve(uri)?
            .local_path
            .ok_or_else(|| "resource provider has no local content path".into())
    }
    /// Let the owning provider open or play its resource when supported.
    pub fn open_with_provider(&self, uri: &str) -> Result<bool, String> {
        self.provider_for_uri(uri)?.open(uri)
    }

    /// Resolve a provider URI. Legacy absolute paths remain readable for existing boards.
    pub fn resolve(&self, uri: &str) -> Result<ResolvedResource, String> {
        if Path::new(uri).is_absolute() && !uri.contains("://") {
            return self.provider("local-file")?.resolve(uri);
        }
        self.provider_for_uri(uri)?.resolve(uri)
    }

    fn provider_for_uri(&self, uri: &str) -> Result<Arc<dyn ResourceProvider>, String> {
        if Path::new(uri).is_absolute() && !uri.contains("://") {
            return self.provider("local-file");
        }
        let providers = self.inner.0.read().map_err(|e| e.to_string())?;
        let matches = providers
            .values()
            .filter(|p| p.accepts(uri))
            .cloned()
            .collect::<Vec<_>>();
        drop(providers);
        match matches.as_slice() {
            [provider] => Ok(Arc::clone(provider)),
            [] => Err(format!("no resource provider accepts URI: {uri}")),
            _ => Err(format!(
                "more than one resource provider accepts URI: {uri}"
            )),
        }
    }

    fn provider(&self, id: &str) -> Result<Arc<dyn ResourceProvider>, String> {
        self.inner
            .0
            .read()
            .map_err(|e| e.to_string())?
            .get(id)
            .cloned()
            .ok_or_else(|| format!("resource provider is unavailable: {id}"))
    }
}

/// Removing the owner feature also removes its resource provider from the shared catalog.
pub struct ResourceProviderRegistration {
    inner: Weak<CatalogInner>,
    id: String,
}
impl Drop for ResourceProviderRegistration {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.upgrade() {
            if let Ok(mut providers) = inner.0.write() {
                providers.remove(&self.id);
            }
        }
    }
}

struct LocalFileProvider;
impl ResourceProvider for LocalFileProvider {
    fn id(&self) -> &'static str {
        "local-file"
    }
    fn accepts(&self, uri: &str) -> bool {
        uri.starts_with("file:")
    }
    fn create_link(&self, source: &str) -> Result<ResourceLink, String> {
        let path = std::fs::canonicalize(source).map_err(|e| e.to_string())?;
        if !path.is_file() {
            return Err("resource source must be a file".into());
        }
        let uri = url::Url::from_file_path(&path)
            .map_err(|_| "file path cannot be represented as a file URI")?
            .to_string();
        self.resolve(&uri).map(|resolved| resolved.link)
    }
    fn resolve(&self, uri: &str) -> Result<ResolvedResource, String> {
        let path = if uri.starts_with("file:") {
            url::Url::parse(uri)
                .map_err(|e| e.to_string())?
                .to_file_path()
                .map_err(|_| "invalid local file URI")?
        } else {
            PathBuf::from(uri)
        };
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if !path.is_file() {
            return Err("linked resource is missing or is not a file".into());
        }
        let title = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("Resource")
            .to_owned();
        let ext = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (kind, mime_type) = match ext.as_str() {
            "png" => (ResourceKind::Image, Some("image/png")),
            "jpg" | "jpeg" => (ResourceKind::Image, Some("image/jpeg")),
            "webp" => (ResourceKind::Image, Some("image/webp")),
            "gif" => (ResourceKind::Image, Some("image/gif")),
            "bmp" => (ResourceKind::Image, Some("image/bmp")),
            "tif" | "tiff" => (ResourceKind::Image, Some("image/tiff")),
            "avif" => (ResourceKind::Image, Some("image/avif")),
            "mp4" | "m4v" => (ResourceKind::Video, Some("video/mp4")),
            "mov" => (ResourceKind::Video, Some("video/quicktime")),
            "mkv" => (ResourceKind::Video, Some("video/x-matroska")),
            "webm" => (ResourceKind::Video, Some("video/webm")),
            "avi" => (ResourceKind::Video, Some("video/x-msvideo")),
            "mp3" => (ResourceKind::Audio, Some("audio/mpeg")),
            "wav" => (ResourceKind::Audio, Some("audio/wav")),
            "flac" => (ResourceKind::Audio, Some("audio/flac")),
            _ => (ResourceKind::File, None),
        };
        let uri = url::Url::from_file_path(&path)
            .map_err(|_| "file path cannot be represented as a file URI")?
            .to_string();
        Ok(ResolvedResource {
            link: ResourceLink {
                uri,
                provider: self.id().into(),
                kind,
                title,
                mime_type: mime_type.map(str::to_owned),
            },
            local_path: Some(path),
        })
    }
}

pub struct MediaResourcePlugin;
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct MediaResourceError(String);
impl Plugin for MediaResourcePlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = MediaResourceError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.media-resources".into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), Self::ApplyError> {
        let _publication = context
            .provide(Arc::new(MediaResourceService::new()))
            .map_err(|e| MediaResourceError(e.to_string()))?;
        Ok(())
    }
}
