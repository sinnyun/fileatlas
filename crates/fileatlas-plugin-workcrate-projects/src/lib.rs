//! Durable WorkCrate project and arbitrary-file reference registry.
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_thumbnails::ThumbnailService;
use fileatlas_workcrate_contracts::{
    WorkCrateFileReference, WorkCratePreview, WorkCrateProject, canvas_image_format,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
use thiserror::Error;
use uuid::Uuid;

const FORMAT_VERSION: u32 = 1;
mod management;
mod portable;
mod storage;
use storage::*;
mod presets;
mod project_presets;
pub use presets::{FieldPreset, FieldPresets};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ProjectIndex {
    #[serde(default = "format_version")]
    version: u32,
    #[serde(default)]
    projects: Vec<WorkCrateProject>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    locations: std::collections::BTreeMap<String, PathBuf>,
}

impl Default for ProjectIndex {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            projects: Vec::new(),
            locations: Default::default(),
        }
    }
}

fn format_version() -> u32 {
    FORMAT_VERSION
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProjectDocument {
    #[serde(default = "format_version")]
    version: u32,
    project_id: String,
    #[serde(default)]
    references: Vec<WorkCrateFileReference>,
    #[serde(default)]
    native_document: Option<PathBuf>,
    #[serde(default)]
    native_binding: String,
    #[serde(default)]
    field_presets: Option<FieldPresets>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    portable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<WorkCrateProject>,
}

impl ProjectDocument {
    fn new(project_id: String) -> Self {
        Self {
            version: FORMAT_VERSION,
            project_id,
            references: Vec::new(),
            native_document: None,
            native_binding: String::new(),
            field_presets: None,
            portable: false,
            project: None,
        }
    }
}

pub struct WorkCrateProjectsService {
    root: PathBuf,
    index: RwLock<ProjectIndex>,
    write_gate: RwLock<()>,
    revision: AtomicU64,
    wake: fileatlas_ui_bridge::refresh::Wake,
    presets: RwLock<FieldPresets>,
    project_presets: RwLock<std::collections::BTreeMap<String, FieldPresets>>,
    locations: RwLock<std::collections::BTreeMap<String, PathBuf>>,
}

impl Service for WorkCrateProjectsService {
    const NAME: &'static str = "fileatlas/workcrate-projects";
}

impl WorkCrateProjectsService {
    pub fn open_at(root: impl Into<PathBuf>) -> Result<Self, WorkCrateProjectsError> {
        let root = root.into();
        let projects_dir = root.join("projects");
        std::fs::create_dir_all(&projects_dir)?;
        let path = root.join("projects.json");
        let backup = path.with_extension("json.bak");
        let (mut index, _recovered_from_backup) = if path.is_file() || backup.is_file() {
            read_json_with_backup(&path)?
        } else {
            (ProjectIndex::default(), false)
        };
        if index.version != FORMAT_VERSION {
            return Err(WorkCrateProjectsError::UnsupportedVersion(index.version));
        }
        validate_index(&index)?;
        let presets = presets::read(&root)?;
        index
            .projects
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let locations = index.locations.clone();
        let service = Self {
            root,
            index: RwLock::new(index),
            write_gate: RwLock::new(()),
            revision: AtomicU64::new(0),
            wake: Default::default(),
            presets: RwLock::new(presets),
            project_presets: RwLock::new(Default::default()),
            locations: RwLock::new(locations),
        };
        for project in service.projects(true)? {
            let document = match service.read_project(&project.id) {
                Ok(document) => document,
                Err(WorkCrateProjectsError::Io(error))
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            if let Some(presets) = document.field_presets {
                presets::validate(&presets)?;
                service
                    .project_presets
                    .write()
                    .map_err(|_| WorkCrateProjectsError::Poisoned)?
                    .insert(project.id, presets);
            }
        }
        Ok(service)
    }

    pub fn open_default() -> Result<Self, WorkCrateProjectsError> {
        let root = dirs::data_local_dir()
            .ok_or(WorkCrateProjectsError::NoDataDirectory)?
            .join("FileAtlas")
            .join("workcrate");
        Self::open_at(root)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the durable storage directory for a known project.
    pub fn project_directory(&self, project_id: &str) -> Result<PathBuf, WorkCrateProjectsError> {
        self.ensure_project(project_id)?;
        self.project_dir(project_id)
    }
    pub fn document_path(
        &self,
        project_id: &str,
    ) -> Result<Option<PathBuf>, WorkCrateProjectsError> {
        self.ensure_project(project_id)?;
        Ok(self.read_project(project_id)?.native_document)
    }
    /// Persisted identity of one explicit native-document association.
    pub fn document_binding(&self, project_id: &str) -> Result<String, WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        self.ensure_project(project_id)?;
        let mut document = self.read_project(project_id)?;
        if document.native_document.is_none() {
            return Err(WorkCrateProjectsError::Invalid(
                "native document not associated",
            ));
        }
        if document.native_binding.is_empty() {
            document.native_binding = Uuid::new_v4().to_string();
            self.write_project(&document)?;
        }
        Ok(document.native_binding)
    }
    pub fn set_document_path(
        &self,
        project_id: &str,
        path: &Path,
    ) -> Result<(), WorkCrateProjectsError> {
        let canonical = std::fs::canonicalize(path)?;
        if !canonical.is_file() {
            return Err(WorkCrateProjectsError::NotAFile(canonical));
        }
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        self.ensure_project(project_id)?;
        let mut document = self.read_project(project_id)?;
        let relocate = self
            .portable_folder(project_id)?
            .is_some_and(|folder| !canonical.starts_with(folder));
        document.native_document = Some(canonical);
        document.native_binding = Uuid::new_v4().to_string();
        if relocate {
            self.publish_relocation(document)?;
        } else {
            self.write_project(&document)?;
        }
        self.changed();
        Ok(())
    }

    /// Clear an association only when it still points to the document created by
    /// the caller, so a failed create flow cannot undo a later user rebind.
    pub fn clear_document_path_if_matches(
        &self,
        project_id: &str,
        expected: &Path,
    ) -> Result<(), WorkCrateProjectsError> {
        let expected = std::fs::canonicalize(expected)?;
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        self.ensure_project(project_id)?;
        let mut document = self.read_project(project_id)?;
        if document.native_document.as_ref() != Some(&expected) {
            return Ok(());
        }
        document.native_document = None;
        document.native_binding.clear();
        self.write_project(&document)?;
        self.changed();
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    pub fn wake(&self) -> fileatlas_ui_bridge::refresh::Wake {
        self.wake.clone()
    }

    pub fn projects(
        &self,
        include_archived: bool,
    ) -> Result<Vec<WorkCrateProject>, WorkCrateProjectsError> {
        let index = self
            .index
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        Ok(index
            .projects
            .iter()
            .filter(|item| include_archived || !item.archived)
            .cloned()
            .collect())
    }

    pub fn create_project(&self, name: &str) -> Result<WorkCrateProject, WorkCrateProjectsError> {
        let name = validate_name(name)?;
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut index = self
            .index
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        ensure_unique_name(&index, &name, None)?;
        let project = WorkCrateProject {
            id: Uuid::new_v4().to_string(),
            name,
            created_at: now(),
            archived: false,
            main_image: None,
        };
        let dir = self.project_dir(&project.id)?;
        std::fs::create_dir_all(dir.join("previews"))?;
        atomic_write_json(
            &dir.join("project.json"),
            &ProjectDocument::new(project.id.clone()),
        )?;
        let mut next = index.clone();
        next.projects.push(project.clone());
        if let Err(error) = atomic_write_json(&self.index_path(), &next) {
            let _ = std::fs::remove_dir_all(dir);
            return Err(error);
        }
        *index = next;
        self.changed();
        Ok(project)
    }

    pub fn rename_project(
        &self,
        id: &str,
        name: &str,
    ) -> Result<WorkCrateProject, WorkCrateProjectsError> {
        let name = validate_name(name)?;
        self.update_index(|index| {
            ensure_unique_name(index, &name, Some(id))?;
            let project = index
                .projects
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| WorkCrateProjectsError::ProjectNotFound(id.to_owned()))?;
            project.name = name;
            Ok(project.clone())
        })
    }

    pub fn set_archived(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<WorkCrateProject, WorkCrateProjectsError> {
        self.update_index(|index| {
            let project = index
                .projects
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| WorkCrateProjectsError::ProjectNotFound(id.to_owned()))?;
            project.archived = archived;
            Ok(project.clone())
        })
    }

    pub fn references(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkCrateFileReference>, WorkCrateProjectsError> {
        self.ensure_project(project_id)?;
        Ok(self.read_project(project_id)?.references)
    }

    pub fn add_references(
        &self,
        project_id: &str,
        paths: &[PathBuf],
    ) -> Result<Vec<WorkCrateFileReference>, WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        self.ensure_project(project_id)?;
        let mut document = self.read_project(project_id)?;
        let before = document.references.len();
        for raw in paths {
            let path = std::fs::canonicalize(raw)?;
            let metadata = std::fs::metadata(&path)?;
            if !metadata.is_file() {
                continue;
            }
            let path_text = path.to_string_lossy().into_owned();
            if document
                .references
                .iter()
                .any(|item| same_path(&item.path, &path_text))
            {
                continue;
            }
            document.references.push(WorkCrateFileReference {
                id: Uuid::new_v4().to_string(),
                display_title: path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("File")
                    .to_owned(),
                path: path_text,
                order: document.references.len() as u32,
                size_bytes: metadata.len(),
                modified_at: modified_at(&metadata),
                custom_preview: None,
            });
        }
        if document.references.len() != before {
            self.write_project(&document)?;
            self.changed();
        }
        Ok(document.references)
    }

    pub fn relink_reference(
        &self,
        project_id: &str,
        asset_id: &str,
        path: &Path,
    ) -> Result<WorkCrateFileReference, WorkCrateProjectsError> {
        let canonical = std::fs::canonicalize(path)?;
        let metadata = std::fs::metadata(&canonical)?;
        if !metadata.is_file() {
            return Err(WorkCrateProjectsError::NotAFile(canonical));
        }
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut document = self.read_project(project_id)?;
        let reference = document
            .references
            .iter_mut()
            .find(|item| item.id == asset_id)
            .ok_or_else(|| WorkCrateProjectsError::ReferenceNotFound(asset_id.to_owned()))?;
        reference.path = canonical.to_string_lossy().into_owned();
        reference.size_bytes = metadata.len();
        reference.modified_at = modified_at(&metadata);
        let result = reference.clone();
        self.write_project(&document)?;
        self.changed();
        Ok(result)
    }

    /// Validate a JPEG/PNG through FileAtlas's shared image thumbnail pipeline,
    /// then copy it into project-owned storage and hash the retained bytes.
    pub fn set_custom_preview(
        &self,
        project_id: &str,
        asset_id: &str,
        path: &Path,
        thumbnails: &ThumbnailService,
    ) -> Result<WorkCratePreview, WorkCrateProjectsError> {
        let canonical = std::fs::canonicalize(path)?;
        if canvas_image_format(&canonical).is_none() {
            return Err(WorkCrateProjectsError::UnsupportedPreviewFormat);
        }
        let (_, width, height, _) = thumbnails
            .generate_image(&canonical)
            .ok_or(WorkCrateProjectsError::InvalidPreview)?;
        let extension = canonical
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut document = self.read_project(project_id)?;
        let reference = document
            .references
            .iter_mut()
            .find(|item| item.id == asset_id)
            .ok_or_else(|| WorkCrateProjectsError::ReferenceNotFound(asset_id.to_owned()))?;
        let file_name = format!("{}.{}", Uuid::new_v4(), extension);
        let preview_dir = self.project_dir(project_id)?.join("previews");
        std::fs::create_dir_all(&preview_dir)?;
        let target = preview_dir.join(&file_name);
        let sha256 = copy_hash_atomic(&canonical, &target)?;
        let preview = WorkCratePreview {
            path: format!("previews/{file_name}"),
            sha256,
            width,
            height,
        };
        reference.custom_preview = Some(preview.clone());
        self.write_project(&document)?;
        self.changed();
        Ok(preview)
    }

    /// Clearing the chosen preview leaves its copied file in place for recovery
    /// and any existing board placement that may still reference it.
    pub fn remove_custom_preview(
        &self,
        project_id: &str,
        asset_id: &str,
    ) -> Result<(), WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut document = self.read_project(project_id)?;
        let reference = document
            .references
            .iter_mut()
            .find(|item| item.id == asset_id)
            .ok_or_else(|| WorkCrateProjectsError::ReferenceNotFound(asset_id.to_owned()))?;
        if reference.custom_preview.take().is_none() {
            return Ok(());
        }
        self.write_project(&document)?;
        self.changed();
        Ok(())
    }

    pub fn remove_reference(
        &self,
        project_id: &str,
        asset_id: &str,
    ) -> Result<(), WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut document = self.read_project(project_id)?;
        let before = document.references.len();
        document.references.retain(|item| item.id != asset_id);
        if before == document.references.len() {
            return Err(WorkCrateProjectsError::ReferenceNotFound(
                asset_id.to_owned(),
            ));
        }
        for (order, item) in document.references.iter_mut().enumerate() {
            item.order = order as u32;
        }
        self.write_project(&document)?;
        self.changed();
        Ok(())
    }

    fn update_index<T>(
        &self,
        update: impl FnOnce(&mut ProjectIndex) -> Result<T, WorkCrateProjectsError>,
    ) -> Result<T, WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut current = self
            .index
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut next = current.clone();
        let result = update(&mut next)?;
        if next == *current {
            return Ok(result);
        }
        self.sync_portable_project_labels(&current, &next)?;
        atomic_write_json(&self.index_path(), &next)?;
        *current = next;
        self.changed();
        Ok(result)
    }

    fn ensure_project(&self, id: &str) -> Result<(), WorkCrateProjectsError> {
        if self
            .index
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?
            .projects
            .iter()
            .any(|p| p.id == id)
        {
            Ok(())
        } else {
            Err(WorkCrateProjectsError::ProjectNotFound(id.to_owned()))
        }
    }
    fn read_project(&self, id: &str) -> Result<ProjectDocument, WorkCrateProjectsError> {
        let path = self.project_dir(id)?.join("project.json");
        let (mut document, _recovered_from_backup): (ProjectDocument, bool) =
            read_json_with_backup(&path)?;
        if document.version != FORMAT_VERSION {
            return Err(WorkCrateProjectsError::UnsupportedVersion(document.version));
        }
        if document.project_id != id {
            return Err(WorkCrateProjectsError::Invalid(
                "project id does not match its directory",
            ));
        }
        if document.portable {
            portable::resolve_paths(
                &mut document,
                path.parent()
                    .and_then(Path::parent)
                    .and_then(Path::parent)
                    .ok_or(WorkCrateProjectsError::Invalid(
                        "portable project has no folder",
                    ))?,
            )?;
        }
        Ok(document)
    }
    fn write_project(&self, document: &ProjectDocument) -> Result<(), WorkCrateProjectsError> {
        let mut stored = document.clone();
        if stored.portable {
            portable::store_paths(
                &mut stored,
                &self
                    .portable_folder(&document.project_id)?
                    .ok_or(WorkCrateProjectsError::Invalid("portable location missing"))?,
            )?;
        }
        atomic_write_json(
            &self.project_dir(&document.project_id)?.join("project.json"),
            &stored,
        )?;
        if let Some(presets) = &document.field_presets {
            self.project_presets
                .write()
                .map_err(|_| WorkCrateProjectsError::Poisoned)?
                .insert(document.project_id.clone(), presets.clone());
        }
        Ok(())
    }
    fn project_dir(&self, id: &str) -> Result<PathBuf, WorkCrateProjectsError> {
        if Uuid::parse_str(id).is_err() {
            return Err(WorkCrateProjectsError::Invalid("project id is not a UUID"));
        }
        if let Some(path) = self
            .locations
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?
            .get(id)
        {
            return Ok(path.join(".workcrate").join(id));
        }
        Ok(self.root.join("projects").join(id))
    }
    fn index_path(&self) -> PathBuf {
        self.root.join("projects.json")
    }
    fn changed(&self) {
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
    }
}

#[derive(Debug, Error)]
pub enum WorkCrateProjectsError {
    #[error("no local application data directory is available")]
    NoDataDirectory,
    #[error("unsupported WorkCrate data version {0}")]
    UnsupportedVersion(u32),
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("file reference not found: {0}")]
    ReferenceNotFound(String),
    #[error("a project named '{0}' already exists")]
    DuplicateName(String),
    #[error("not a file: {0}")]
    NotAFile(PathBuf),
    #[error("custom preview must be a supported JPEG or PNG image")]
    UnsupportedPreviewFormat,
    #[error("custom preview could not be decoded as an image")]
    InvalidPreview,
    #[error("invalid WorkCrate data: {0}")]
    Invalid(&'static str),
    #[error("WorkCrate data lock was poisoned")]
    Poisoned,
    #[error("plugin dependency or service publication failed: {0}")]
    Plugin(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Default)]
pub struct WorkCrateProjectsConfig {
    pub root: Option<PathBuf>,
}

pub struct WorkCrateProjectsPlugin;
impl Plugin for WorkCrateProjectsPlugin {
    type Config = WorkCrateProjectsConfig;
    type Input = WorkCrateProjectsConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = WorkCrateProjectsError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.workcrate-projects".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let service = Arc::new(match &config.root {
            Some(path) => WorkCrateProjectsService::open_at(path)?,
            None => WorkCrateProjectsService::open_default()?,
        });
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| WorkCrateProjectsError::Plugin(error.to_string()))?;
        let event_service = Arc::clone(&service);
        let _add_references_listener = context
            .on::<fileatlas_workcrate_contracts::WorkCrateAddReferencesRequested, _>(
                cordis::event::observer_sync(
                    move |_, request: fileatlas_workcrate_contracts::WorkCrateAddReferencesRequest| {
                    let service = Arc::clone(&event_service);
                    tokio::spawn(async move {
                        let paths = request
                            .paths
                            .into_iter()
                            .map(PathBuf::from)
                            .collect::<Vec<_>>();
                        match tokio::task::spawn_blocking(move || {
                            service.add_references(&request.project_id, &paths)
                        })
                        .await
                        {
                            Ok(Ok(_)) => {}
                            Ok(Err(error)) => {
                                tracing::warn!(%error, "could not add WorkCrate file references")
                            }
                            Err(error) => {
                                tracing::warn!(%error, "WorkCrate file reference worker failed")
                            }
                        }
                    });
                    Ok::<_, std::convert::Infallible>(())
                    },
                ),
            )
            .map_err(|error| WorkCrateProjectsError::Plugin(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod portable_tests;
#[cfg(test)]
mod tests;
