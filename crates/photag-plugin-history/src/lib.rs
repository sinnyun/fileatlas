#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Source-compatible undo/redo history for PhoTag's currently supported actions.

use std::sync::{Arc, Mutex};

use cordis::event::observer_sync;
use cordis::{Context, Plugin, Routing, Service};
use photag_contracts::MediaIndexChanged;
use photag_plugin_database::{DatabaseService, db::queries};
use photag_plugin_file_operations::FileOperationsService;
use photag_plugin_tagging::TaggingService;
use photag_ui_bridge::{ContributionKind, UiCommandInvoked, UiContribution, register_for_plugin};

const HISTORY_LIMIT: usize = 100;
const UNDO_COMMAND_ID: &str = "history/undo";
const REDO_COMMAND_ID: &str = "history/redo";

/// A reversible operation recorded by the original PhoTag application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoEntry {
    AssignedTags {
        media_ids: Vec<i64>,
        tag_ids: Vec<i64>,
    },
    RemovedTags {
        media_ids: Vec<i64>,
        tag_ids: Vec<i64>,
    },
    ClearedTags {
        media_ids: Vec<i64>,
        previous_tags: Vec<(i64, Vec<i64>)>,
    },
    RatingChanged {
        media_ids: Vec<i64>,
        old_rating: i32,
        new_rating: i32,
    },
    Renamed {
        media_id: i64,
        old_filename: String,
        new_filename: String,
    },
}

#[derive(Debug, Default)]
struct Stacks {
    undo: Vec<UndoEntry>,
    redo: Vec<UndoEntry>,
}

/// Snapshot-owning history service. Operation failures intentionally do not
/// prevent stack movement, matching PhoTag's original best-effort handlers.
pub struct HistoryService {
    database: Arc<DatabaseService>,
    tagging: Arc<TaggingService>,
    files: Arc<FileOperationsService>,
    stacks: Mutex<Stacks>,
}

impl Service for HistoryService {
    const NAME: &'static str = "photag/history";
}

impl HistoryService {
    pub fn new(
        database: Arc<DatabaseService>,
        tagging: Arc<TaggingService>,
        files: Arc<FileOperationsService>,
    ) -> Self {
        Self {
            database,
            tagging,
            files,
            stacks: Mutex::new(Stacks::default()),
        }
    }

    /// Record a completed operation; a fresh operation always discards redo.
    #[tracing::instrument(skip_all)]
    pub fn push(&self, entry: UndoEntry) {
        let Ok(mut stacks) = self.stacks.lock() else {
            return;
        };
        stacks.redo.clear();
        stacks.undo.push(entry);
        if stacks.undo.len() > HISTORY_LIMIT {
            stacks.undo.remove(0);
        }
        let ids = stacks
            .undo
            .iter()
            .chain(stacks.redo.iter())
            .flat_map(|entry| match entry {
                UndoEntry::Renamed { media_id, .. } => vec![*media_id],
                UndoEntry::AssignedTags { media_ids, .. }
                | UndoEntry::RemovedTags { media_ids, .. }
                | UndoEntry::ClearedTags { media_ids, .. }
                | UndoEntry::RatingChanged { media_ids, .. } => media_ids.clone(),
            })
            .collect::<Vec<_>>();
        if let Err(error) = self.database.pin_history(&ids) {
            tracing::error!(%error,"history cache pin failed");
        }
    }

    /// Capture the exact current assignments before the caller clears tags.
    pub fn cleared_tags_entry(&self, media_ids: Vec<i64>) -> Result<UndoEntry, HistoryError> {
        let tags = self
            .database
            .with_reader(|connection| queries::get_tags_for_media_batch(connection, &media_ids))?;
        let previous_tags = media_ids
            .iter()
            .map(|id| {
                let ids = tags
                    .get(id)
                    .into_iter()
                    .flatten()
                    .map(|tag| tag.id)
                    .collect();
                (*id, ids)
            })
            .collect();
        Ok(UndoEntry::ClearedTags {
            media_ids,
            previous_tags,
        })
    }

    pub fn undo_description(&self) -> Option<String> {
        self.stacks.lock().ok()?.undo.last().map(describe)
    }

    pub fn redo_description(&self) -> Option<String> {
        self.stacks.lock().ok()?.redo.last().map(describe)
    }

    /// Apply the inverse of the latest action. Returns the source-compatible toast text.
    #[tracing::instrument(skip_all)]
    pub async fn undo(&self) -> Option<String> {
        let entry = self.stacks.lock().ok()?.undo.pop()?;
        let message = format!("Undo: {}", describe(&entry));
        match &entry {
            UndoEntry::AssignedTags { media_ids, tag_ids } => {
                let _ = self.tagging.remove(media_ids, tag_ids);
            }
            UndoEntry::RemovedTags { media_ids, tag_ids } => {
                let _ = self.tagging.restore_assignment(media_ids, tag_ids);
            }
            UndoEntry::ClearedTags { previous_tags, .. } => {
                for (media_id, tag_ids) in previous_tags {
                    if !tag_ids.is_empty() {
                        let _ = self.tagging.restore_assignment(&[*media_id], tag_ids);
                    }
                }
            }
            UndoEntry::RatingChanged {
                media_ids,
                old_rating,
                ..
            } => {
                let _ = self.database.with_writer(|connection| {
                    queries::set_rating_bulk(connection, media_ids, *old_rating)
                });
            }
            UndoEntry::Renamed {
                media_id,
                old_filename,
                new_filename,
            } => {
                let _ = self.files.rename(*media_id, old_filename.clone()).await;
                let inverted = UndoEntry::Renamed {
                    media_id: *media_id,
                    old_filename: new_filename.clone(),
                    new_filename: old_filename.clone(),
                };
                if let Ok(mut stacks) = self.stacks.lock() {
                    stacks.redo.push(inverted);
                }
                return Some(message);
            }
        }
        if let Ok(mut stacks) = self.stacks.lock() {
            stacks.redo.push(entry);
        }
        Some(message)
    }

    /// Re-apply the latest undone action.
    #[tracing::instrument(skip_all)]
    pub async fn redo(&self) -> Option<String> {
        let entry = self.stacks.lock().ok()?.redo.pop()?;
        let message = format!("Redo: {}", describe(&entry));
        match &entry {
            UndoEntry::AssignedTags { media_ids, tag_ids } => {
                let _ = self.tagging.restore_assignment(media_ids, tag_ids);
            }
            UndoEntry::RemovedTags { media_ids, tag_ids } => {
                let _ = self.tagging.remove(media_ids, tag_ids);
            }
            UndoEntry::ClearedTags { media_ids, .. } => {
                let _ = self.tagging.clear(media_ids);
            }
            UndoEntry::RatingChanged {
                media_ids,
                new_rating,
                ..
            } => {
                let _ = self.database.with_writer(|connection| {
                    queries::set_rating_bulk(connection, media_ids, *new_rating)
                });
            }
            UndoEntry::Renamed {
                media_id,
                old_filename,
                new_filename,
            } => {
                let _ = self.files.rename(*media_id, new_filename.clone()).await;
                let forward = UndoEntry::Renamed {
                    media_id: *media_id,
                    old_filename: old_filename.clone(),
                    new_filename: new_filename.clone(),
                };
                if let Ok(mut stacks) = self.stacks.lock() {
                    stacks.undo.push(forward);
                }
                return Some(message);
            }
        }
        if let Ok(mut stacks) = self.stacks.lock() {
            stacks.undo.push(entry);
        }
        Some(message)
    }
}

fn describe(entry: &UndoEntry) -> String {
    match entry {
        UndoEntry::AssignedTags { media_ids, tag_ids } => format!(
            "Assigned {} tag{} to {} item{}",
            tag_ids.len(),
            plural(tag_ids.len()),
            media_ids.len(),
            plural(media_ids.len())
        ),
        UndoEntry::RemovedTags { media_ids, tag_ids } => format!(
            "Removed {} tag{} from {} item{}",
            tag_ids.len(),
            plural(tag_ids.len()),
            media_ids.len(),
            plural(media_ids.len())
        ),
        UndoEntry::ClearedTags { media_ids, .. } => format!(
            "Cleared tags from {} item{}",
            media_ids.len(),
            plural(media_ids.len())
        ),
        UndoEntry::RatingChanged {
            old_rating,
            new_rating,
            ..
        } => format!("Rating {} → {}", old_rating, new_rating),
        UndoEntry::Renamed {
            old_filename,
            new_filename,
            ..
        } => format!("Renamed \"{}\" → \"{}\"", old_filename, new_filename),
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error(transparent)]
    Database(#[from] photag_plugin_database::DatabaseAccessError),
}

#[derive(Debug, Clone, Default)]
pub struct HistoryConfig;
pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    type Config = HistoryConfig;
    type Input = HistoryConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = HistoryPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.history".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|e| HistoryPluginError::Dependency(e.to_string()))?;
        let tagging = context
            .try_service::<TaggingService>()
            .map_err(|e| HistoryPluginError::Dependency(e.to_string()))?;
        let files = context
            .try_service::<FileOperationsService>()
            .map_err(|e| HistoryPluginError::Dependency(e.to_string()))?;
        let service = Arc::new(HistoryService::new(database, tagging, files));
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|e| HistoryPluginError::Publish(e.to_string()))?;
        let command_service = Arc::clone(&service);
        let cordis = context.clone();
        let runtime = tokio::runtime::Handle::current();
        let _command_listener = context
            .on::<UiCommandInvoked, _>(observer_sync(move |_, command_id: String| {
                let undo = match command_id.as_str() {
                    UNDO_COMMAND_ID => true,
                    REDO_COMMAND_ID => false,
                    _ => return Ok::<_, std::convert::Infallible>(()),
                };
                let service = Arc::clone(&command_service);
                let cordis = cordis.clone();
                runtime.spawn(async move {
                    let result = if undo {
                        service.undo().await
                    } else {
                        service.redo().await
                    };
                    if result.is_some()
                        && let Err(error) = cordis
                            .emit::<MediaIndexChanged>(Routing::Unscoped, ())
                            .await
                    {
                        tracing::error!("history refresh event failed: {error}");
                    }
                });
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|e| HistoryPluginError::EventRegistration(e.to_string()))?;
        register_for_plugin(
            &context,
            "photag.history",
            [
                UiContribution::new(
                    UNDO_COMMAND_ID,
                    ContributionKind::Command {
                        label: "Undo".into(),
                    },
                ),
                UiContribution::new(
                    "history/undo-menu-item",
                    ContributionKind::MenuItem {
                        label: "Undo".into(),
                        menu: "Edit".into(),
                        command_id: UNDO_COMMAND_ID.into(),
                        order: 10,
                    },
                ),
                UiContribution::new(
                    REDO_COMMAND_ID,
                    ContributionKind::Command {
                        label: "Redo".into(),
                    },
                ),
                UiContribution::new(
                    "history/redo-menu-item",
                    ContributionKind::MenuItem {
                        label: "Redo".into(),
                        menu: "Edit".into(),
                        command_id: REDO_COMMAND_ID.into(),
                        order: 20,
                    },
                ),
            ],
        )
        .map_err(|e| HistoryPluginError::UiRegistration(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryPluginError {
    #[error("required history dependency unavailable: {0}")]
    Dependency(String),
    #[error("history service publication failed: {0}")]
    Publish(String),
    #[error("history command listener registration failed: {0}")]
    EventRegistration(String),
    #[error("history UI registration failed: {0}")]
    UiRegistration(String),
}
