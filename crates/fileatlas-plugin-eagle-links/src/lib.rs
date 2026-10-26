//! Convert Eagle's local file URLs into FileAtlas canvas resource links.
use cordis::{Context, Plugin, Service};
use fileatlas_contracts::resources::ResourceLink;
use fileatlas_plugin_media_resources::MediaResourceService;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use url::Url;

pub struct EagleLinkService {
    resources: Arc<MediaResourceService>,
}
impl Service for EagleLinkService {
    const NAME: &'static str = "fileatlas/eagle-links";
}
impl EagleLinkService {
    /// Resolve an Eagle Plugin API `fileURL` (or its `filePath`) into the same
    /// canonical `file://` ResourceLink used by canvas imports and drag/drop.
    pub fn convert(&self, eagle_location: &str) -> Result<ResourceLink, String> {
        let value = eagle_location.trim();
        if value.is_empty() {
            return Err("Eagle file location is empty".into());
        }
        let path = if value.starts_with("file:") {
            url::Url::parse(value)
                .map_err(|e| format!("invalid Eagle file URL: {e}"))?
                .to_file_path()
                .map_err(|_| "Eagle URL must identify a local file")?
        } else {
            std::path::PathBuf::from(value)
        };
        self.resources
            .link_file(path)
            .map_err(|e| format!("cannot link Eagle file to canvas: {e}"))
    }

    /// Resolve an Eagle item through its running local API and link the original
    /// file stored beside the item's thumbnail.
    pub fn link_item_by_id(&self, item_id: &str) -> Result<ResourceLink, String> {
        self.resources.link_file(resolve_original(item_id)?)
    }

    /// Ask the registered Eagle URL handler to open the library item by ID.
    pub fn open_item(&self, item_id: &str) -> Result<(), String> {
        let item_id = item_id.trim();
        if item_id.is_empty() || !item_id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err("Eagle item ID is invalid".into());
        }
        open::that_detached(format!("eagle://item/{item_id}"))
            .map_err(|error| format!("cannot open Eagle item {item_id}: {error}"))
    }

    /// Attach Eagle API metadata only when the dropped file is confirmed as
    /// the original file represented by the containing `.info` item folder.
    pub fn source_for_dropped_image(
        &self,
        path: &Path,
    ) -> Result<Option<EagleImageSource>, String> {
        let canonical = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        let Some(item_id) = canonical
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".info"))
        else {
            return Ok(None);
        };
        let item = eagle_item(item_id)?;
        let original =
            std::fs::canonicalize(resolve_original(item_id)?).map_err(|e| e.to_string())?;
        if canonical != original {
            return Ok(None);
        }
        let thumbnail = eagle_thumbnail_path(item_id)?;
        let api_url = item_api_url(item_id)?;
        let folders = eagle_folders(&item.folders)?;
        let link = self.resources.link_file(&original)?;
        Ok(Some(EagleImageSource {
            item_id: item.id,
            api_url,
            name: item.name,
            original_path: original.to_string_lossy().into_owned(),
            original_uri: link.uri,
            thumbnail_path: thumbnail.to_string_lossy().into_owned(),
            folders,
        }))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EagleImageSource {
    pub item_id: String,
    pub api_url: String,
    pub name: String,
    pub original_path: String,
    pub original_uri: String,
    pub thumbnail_path: String,
    pub folders: Vec<EagleFolderSource>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EagleFolderSource {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize)]
struct Envelope<T> {
    status: String,
    data: T,
    #[serde(default)]
    message: Option<String>,
}
#[derive(Deserialize)]
struct ItemPage {
    data: Vec<EagleItem>,
}
#[derive(Deserialize)]
struct FolderPage {
    data: Vec<EagleFolder>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EagleItem {
    id: String,
    name: String,
    ext: String,
    size: u64,
    #[serde(default)]
    folders: Vec<String>,
    #[serde(default)]
    is_deleted: bool,
    #[serde(default)]
    no_thumbnail: bool,
}
#[derive(Deserialize)]
struct EagleFolder {
    id: String,
    name: String,
    #[serde(default)]
    children: Vec<EagleFolder>,
}

fn request_json<T: for<'de> Deserialize<'de>>(url: &Url) -> Result<T, String> {
    let response = ureq::get(url.as_str())
        .call()
        .map_err(|e| format!("Eagle Web API request failed: {e}"))?;
    let envelope: Envelope<T> = response
        .into_json()
        .map_err(|e| format!("invalid Eagle Web API response: {e}"))?;
    if envelope.status != "success" {
        return Err(envelope
            .message
            .unwrap_or_else(|| "Eagle Web API returned an error".into()));
    }
    Ok(envelope.data)
}

fn eagle_item(item_id: &str) -> Result<EagleItem, String> {
    let item_id = item_id.trim();
    if item_id.is_empty() {
        return Err("Eagle item ID is empty".into());
    }
    let mut url =
        Url::parse("http://127.0.0.1:41595/api/v2/item/get").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("id", item_id);
    let page: ItemPage = request_json(&url)?;
    match page.data.as_slice() {
        [item] if !item.is_deleted => Ok(EagleItem {
            id: item.id.clone(),
            name: item.name.clone(),
            ext: item.ext.clone(),
            size: item.size,
            folders: item.folders.clone(),
            is_deleted: false,
            no_thumbnail: item.no_thumbnail,
        }),
        [] => Err("Eagle item was not found".into()),
        [_] => Err("Eagle item is in the trash".into()),
        _ => Err("Eagle returned multiple items for one ID".into()),
    }
}

fn item_api_url(item_id: &str) -> Result<String, String> {
    let mut url =
        Url::parse("http://127.0.0.1:41595/api/v2/item/get").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("id", item_id);
    Ok(url.to_string())
}

fn eagle_folders(ids: &[String]) -> Result<Vec<EagleFolderSource>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut url =
        Url::parse("http://127.0.0.1:41595/api/v2/folder/get").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("ids", &ids.join(","));
    let page: FolderPage = request_json(&url)?;
    fn flatten(folders: Vec<EagleFolder>, ids: &[String], result: &mut Vec<EagleFolderSource>) {
        for folder in folders {
            if ids.iter().any(|id| id == &folder.id) {
                result.push(EagleFolderSource {
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                });
            }
            flatten(folder.children, ids, result);
        }
    }
    let mut result = Vec::new();
    flatten(page.data, ids, &mut result);
    Ok(result)
}

fn resolve_original(item_id: &str) -> Result<PathBuf, String> {
    let item = eagle_item(item_id)?;
    let thumbnail = eagle_thumbnail_path(item_id)?;
    if item.no_thumbnail {
        return Ok(thumbnail);
    }
    let parent = thumbnail
        .parent()
        .ok_or("Eagle thumbnail path has no parent directory")?;
    let expected_dir = format!("{}.info", item.id);
    if !parent
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(&expected_dir))
    {
        return Err("Eagle thumbnail is not in the expected item directory".into());
    }
    let mut matches = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if !path.is_file() || path == thumbnail {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if ext.eq_ignore_ascii_case(&item.ext)
            && std::fs::metadata(&path).map_err(|e| e.to_string())?.len() == item.size
        {
            matches.push(path);
        }
    }
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err("could not identify Eagle original beside its thumbnail".into()),
        _ => Err("multiple Eagle sibling files match the original item".into()),
    }
}

fn eagle_thumbnail_path(item_id: &str) -> Result<std::path::PathBuf, String> {
    let mut url =
        Url::parse("http://127.0.0.1:41595/api/item/thumbnail").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("id", item_id.trim());
    let encoded_path: String = request_json(&url)?;
    let decoded_path = percent_encoding::percent_decode_str(&encoded_path)
        .decode_utf8()
        .map_err(|e| format!("invalid Eagle thumbnail path encoding: {e}"))?;
    let path = std::path::PathBuf::from(decoded_path.as_ref());
    if !path.is_file() {
        return Err("Eagle thumbnail file is unavailable".into());
    }
    Ok(path)
}

pub struct EagleLinksPlugin;
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EagleLinksError(String);
impl Plugin for EagleLinksPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = EagleLinksError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.eagle-links".into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), Self::ApplyError> {
        let resources = context
            .try_service::<MediaResourceService>()
            .map_err(|e| EagleLinksError(e.to_string()))?;
        let _publication = context
            .provide(Arc::new(EagleLinkService { resources }))
            .map_err(|e| EagleLinksError(e.to_string()))?;
        Ok(())
    }
}
