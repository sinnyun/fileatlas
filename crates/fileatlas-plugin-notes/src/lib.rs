//! Markdown-first personal notes integrated as a Cordis feature plugin.

use std::{path::PathBuf, sync::Arc};

use base64::Engine as _;
use cordis::{Context, Plugin, Service};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod mcp;
mod editor {
    pub mod commands;
    pub mod markdown_syntax;
}
mod view;

const OWNER: &str = "fileatlas.notes";
const SERVICE: &str = "fileatlas/notes";
const PANEL: &str = "notes/workbench";

#[derive(Debug, Error)]
pub enum NotesError {
    #[error("note storage is unavailable: {0}")]
    Io(#[from] std::io::Error),
    #[error("note data is malformed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("note not found: {0}")]
    NotFound(String),
    #[error("a note or folder with that name already exists")]
    AlreadyExists,
    #[error("invalid note title or path")]
    InvalidName,
    #[error("notes service failed: {0}")]
    Plugin(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteMetadata {
    pub id: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_status() -> String {
    "draft".into()
}

impl Default for NoteMetadata {
    fn default() -> Self {
        Self {
            id: String::new(),
            category: String::new(),
            status: default_status(),
            tags: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub path: String,
    pub category: String,
    pub status: String,
    pub tags: Vec<String>,
    pub modified_unix_ms: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteDocument {
    pub summary: NoteSummary,
    pub body: String,
    pub backlinks: Vec<NoteSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteFolder {
    pub name: String,
    pub path: String,
    pub depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteHeading {
    pub level: usize,
    pub title: String,
    pub line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteTrashEntry {
    pub path: String,
    pub name: String,
    pub is_folder: bool,
    pub note_count: usize,
}

mod service;
pub use service::NotesService;
pub struct NotesConfig {
    pub root: Option<PathBuf>,
}
impl Default for NotesConfig {
    fn default() -> Self {
        Self { root: None }
    }
}

#[derive(Clone, Copy)]
pub struct NotesPlugin;
impl Plugin for NotesPlugin {
    type Config = NotesConfig;
    type Input = PathBuf;
    type PrepareError = NotesError;
    type ApplyError = NotesError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, config: NotesConfig) -> Result<PathBuf, NotesError> {
        Ok(config.root.unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("FileAtlas")
                .join("notes")
        }))
    }
    async fn apply(&self, context: Context, root: &PathBuf) -> Result<(), NotesError> {
        let service = NotesService::open(root.clone())?;
        let publication = context
            .provide(Arc::clone(&service))
            .map_err(|e| NotesError::Plugin(e.to_string()))?;
        let bridge = context
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .map_err(|e| NotesError::Plugin(e.to_string()))?;
        let _panel = fileatlas_ui_bridge::panel_catalog::register_panel_for_plugin(
            &context,
            OWNER,
            fileatlas_ui_bridge::panel_catalog::PanelDefinition {
                options: fileatlas_ui_bridge::panels::PanelOptions {
                    id: PANEL.into(),
                    title: "笔记".into(),
                    modal: false,
                    default_size: (1020., 700.),
                    min_size: (620., 420.),
                },
                multiplicity: fileatlas_ui_bridge::panel_catalog::PanelMultiplicity::Single,
                factory: Arc::new({
                    let service = Arc::clone(&service);
                    move |window, app| view::panel(Arc::clone(&service), window, app)
                }),
            },
        )
        .map_err(|e| NotesError::Plugin(e.to_string()))?;
        fileatlas_ui_bridge::register_views_for_plugin(
            &context,
            OWNER,
            [view::sidebar(
                Arc::clone(&service),
                Arc::clone(&bridge.panels),
            )],
        )
        .map_err(|e| NotesError::Plugin(e.to_string()))?;
        mcp::register(&context, &service)?;
        let _publication = publication;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::new_id;
    use std::fs;
    #[test]
    fn markdown_files_support_nested_notes_tags_links_and_trash() {
        let temp = std::env::temp_dir().join(format!("fileatlas-notes-test-{}", new_id()));
        let service = NotesService::open(temp.clone()).unwrap();
        let folder = service.create_folder(None, "Research").unwrap();
        fs::write(
            temp.join("Legacy.md"),
            "---\n{\"id\":\"legacy\",\"category\":\"\",\"status\":\"\",\"tags\":[]}\n---\nLegacy body\n",
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            service
                .read("Legacy.md")
                .unwrap_or_else(|error| panic!("{error}"))
                .summary
                .status,
            "draft"
        );
        let mut deep_folder = folder.path.clone();
        for level in 0..24 {
            deep_folder = service
                .create_folder(Some(&deep_folder), &format!("Level-{level}"))
                .unwrap_or_else(|error| panic!("{error}"))
                .path;
        }
        let deep_note = service
            .create("Deep note", Some(&deep_folder), Some("Nested deeply"))
            .unwrap_or_else(|error| panic!("{error}"));
        let deep_summary = service
            .list_folders()
            .unwrap_or_else(|error| panic!("{error}"))
            .into_iter()
            .find(|candidate| candidate.path == deep_folder)
            .unwrap_or_else(|| panic!("deep folder is missing"));
        assert_eq!(deep_summary.depth, 24);
        let target = service
            .create("Target", Some(&folder.path), Some("# Target"))
            .unwrap();
        assert_eq!(target.status, "draft");
        let source = service
            .create("Source", None, Some("See [[Target]]"))
            .unwrap();
        let image_source = temp.join("fixture.png");
        fs::write(&image_source, b"image bytes").unwrap();
        let image_markdown = service.insert_image(&source.id, &image_source).unwrap();
        service
            .update(
                &source.id,
                &format!("See [[Target]]\n\n{image_markdown}"),
                Some("Article"),
                Some("completed"),
                Some(vec!["rust".into(), "notes".into()]),
            )
            .unwrap();
        assert_eq!(
            service.read(&source.id).unwrap().summary.category,
            "Article"
        );
        assert_eq!(service.backlinks(&target.id).unwrap().len(), 1);
        let moved = service.move_note(&source.id, Some(&folder.path)).unwrap();
        assert!(moved.path.starts_with("Research/"));
        assert!(
            service
                .rendered_markdown(&source.id)
                .unwrap()
                .contains("data:image/png;base64,")
        );
        assert_eq!(service.search("notes").unwrap().len(), 1);
        service.rename(&target.id, "Renamed").unwrap();
        assert!(
            fs::read_to_string(temp.join("Research").join("Source.md"))
                .unwrap()
                .contains("[[Renamed]]")
        );
        let trashed = service.trash(&source.id).unwrap();
        assert!(trashed.exists());
        let trash_path = service
            .list_trash()
            .unwrap_or_else(|error| panic!("{error}"))[0]
            .path
            .clone();
        let restored = service
            .restore(&trash_path, None)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(restored.title, "Source");
        let folder_trash = service
            .trash_folder(&folder.path)
            .unwrap_or_else(|error| panic!("{error}"));
        let entries = service
            .list_trash()
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == folder_trash && entry.is_folder)
        );
        let restored_folder = service
            .restore_folder(&folder_trash, None)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(restored_folder.name, "Research");
        assert!(service.read(&deep_note.id).is_ok());
        let source_asset_dir = temp.join("assets").join(&restored.id);
        assert!(source_asset_dir.exists());
        let trash_again = service
            .trash(&restored.id)
            .unwrap_or_else(|error| panic!("{error}"));
        let trash_again = format!(
            ".trash/{}",
            trash_again
                .file_name()
                .unwrap_or_else(|| panic!("missing trash filename"))
                .to_string_lossy()
        );
        service
            .delete_trash(&trash_again)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(!source_asset_dir.exists());
        assert!(
            service
                .list_trash()
                .unwrap_or_else(|error| panic!("{error}"))
                .is_empty()
        );
        let _ = fs::remove_dir_all(temp);
    }
}
