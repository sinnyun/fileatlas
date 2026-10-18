//! PhoTag media-tag assignments and tag associations, owned by Cordis.

use std::sync::{Arc, Mutex};

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::{self, Tag, TagDropPos};
use photag_ui_bridge::{ContributionKind, UiContribution, register_for_plugin};

mod records;
#[cfg(test)]
mod records_tests;
mod writeback;
pub mod xmp;
pub use writeback::{ScheduleError, WritebackFailure};

pub struct TaggingService {
    database: Arc<DatabaseService>,
    thumbnails: Option<Arc<photag_plugin_thumbnails::ThumbnailService>>,
    recent_tag_ids: Mutex<Vec<i64>>,
}

impl Service for TaggingService {
    const NAME: &'static str = "photag/tagging";
}

impl TaggingService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self {
            database,
            thumbnails: None,
            recent_tag_ids: Mutex::new(Vec::new()),
        }
    }

    /// Assign source tags, their one-way associated tags, and required ancestors.
    /// The database query preserves PhoTag's manual-versus-auto-applied flags.
    #[tracing::instrument(skip_all, err)]
    pub fn assign(&self, media_ids: &[i64], tag_ids: &[i64]) -> Result<(), TaggingError> {
        self.assign_impl(media_ids, tag_ids, true)
    }

    /// Restore an assignment while replaying history without changing recency.
    #[tracing::instrument(skip_all, err)]
    pub fn restore_assignment(
        &self,
        media_ids: &[i64],
        tag_ids: &[i64],
    ) -> Result<(), TaggingError> {
        self.assign_impl(media_ids, tag_ids, false)
    }

    fn assign_impl(
        &self,
        media_ids: &[i64],
        tag_ids: &[i64],
        track_recent: bool,
    ) -> Result<(), TaggingError> {
        self.capture_sources(media_ids)?;
        self.database.assign_tags(media_ids, tag_ids)?;
        if track_recent && let Ok(mut recent) = self.recent_tag_ids.lock() {
            for &tag_id in tag_ids.iter().rev() {
                recent.retain(|id| *id != tag_id);
                recent.insert(0, tag_id);
            }
            recent.truncate(8);
        }
        Ok(())
    }

    /// Recently assigned tag IDs, newest first, shared by Gallery and Preview.
    pub fn recent_tags(&self) -> Vec<i64> {
        self.recent_tag_ids
            .lock()
            .map(|recent| recent.clone())
            .unwrap_or_default()
    }

    /// Snapshot metadata write failures owned by this service.
    pub fn writeback_failures(&self) -> Vec<WritebackFailure> {
        writeback::failures()
    }

    /// Retry the exact payload retained for a failed media path.
    pub fn retry_writeback_failure(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<bool, ScheduleError> {
        writeback::retry_failure(path)
    }

    /// Dismiss the retained writeback failure for a media path.
    pub fn dismiss_writeback_failure(&self, path: impl AsRef<std::path::Path>) {
        writeback::clear_failure(path);
    }

    #[tracing::instrument(skip_all, err)]
    pub fn remove(&self, media_ids: &[i64], tag_ids: &[i64]) -> Result<(), TaggingError> {
        self.database.remove_tags(media_ids, tag_ids)?;
        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    pub fn clear(&self, media_ids: &[i64]) -> Result<(), TaggingError> {
        self.database.remove_all_tags(media_ids)?;
        Ok(())
    }

    /// Create missing AI suggestion tags beneath PhoTag's source-compatible
    /// `AI Suggested` group and assign all requested labels to one item.
    #[tracing::instrument(skip_all, err)]
    pub fn apply_ai_suggestions(
        &self,
        media_id: i64,
        labels: &[String],
    ) -> Result<Vec<i64>, TaggingError> {
        let tag_ids = self.database.create_or_find_ai_tags(labels)?;
        self.assign(&[media_id], &tag_ids)?;
        Ok(tag_ids)
    }

    pub fn for_media(&self, media_id: i64) -> Result<Vec<Tag>, TaggingError> {
        Ok(self.database.get_media_tags(media_id)?)
    }

    /// Return the current hierarchical tag tree for assignment pickers.
    pub fn tag_tree(&self) -> Result<Vec<Tag>, TaggingError> {
        Ok(self.database.get_tag_tree()?)
    }

    pub fn associated_with(&self, tag_id: i64) -> Result<Vec<i64>, TaggingError> {
        Ok(self.database.get_associated_tags(tag_id)?)
    }

    #[tracing::instrument(skip_all, err)]
    pub fn set_associations(
        &self,
        tag_id: i64,
        associated_ids: &[i64],
    ) -> Result<(), TaggingError> {
        Ok(self.database.set_associated_tags(tag_id, associated_ids)?)
    }

    #[tracing::instrument(skip_all, err)]
    pub fn merge(
        &self,
        source_id: i64,
        target_id: i64,
        reparent_children: bool,
    ) -> Result<(), TaggingError> {
        self.database
            .merge_tag(source_id, target_id, reparent_children)?;
        Ok(())
    }

    /// Reparent a tag and let the query report media whose implicit ancestor
    /// assignments changed; those media receive a fresh metadata write.
    #[tracing::instrument(skip_all, err)]
    pub fn move_tag(
        &self,
        source_id: i64,
        drop_position: TagDropPos,
    ) -> Result<Vec<i64>, TaggingError> {
        let affected = self.database.move_tag(source_id, drop_position)?;
        Ok(affected)
    }

    /// Explicitly write database tags to file metadata when requested by an
    /// integration. Normal tag edits remain database-only.
    pub fn schedule_xmp_sync(&self, media_ids: &[i64]) {
        if media_ids.is_empty() {
            return;
        }
        if let Err(error) = writeback::schedule(&self.database, media_ids) {
            tracing::error!("tag metadata writeback could not be scheduled: {error}");
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaggingError {
    #[error(transparent)]
    DatabaseAccess(#[from] photag_plugin_database::DatabaseAccessError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct TaggingConfig;
pub struct TaggingPlugin;

impl Plugin for TaggingPlugin {
    type Config = TaggingConfig;
    type Input = TaggingConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = TaggingPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.tagging".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| TaggingPluginError::Dependency(error.to_string()))?;
        let mut service = TaggingService::new(database);
        service.thumbnails = context
            .try_service::<photag_plugin_thumbnails::ThumbnailService>()
            .ok();
        let service = Arc::new(service);
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| TaggingPluginError::Publish(error.to_string()))?;
        register_tagging_mcp_tools(&context, &service)?;
        register_for_plugin(
            &context,
            "photag.tagging",
            [UiContribution::new(
                "tagging/assignment-widget",
                ContributionKind::Widget {
                    label: "Tag Assignments".into(),
                    slot: "gallery.selection-actions".into(),
                    order: 20,
                },
            )],
        )
        .map_err(|error| TaggingPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

fn register_tagging_mcp_tools(
    context: &Context,
    tagging: &Arc<TaggingService>,
) -> Result<(), TaggingPluginError> {
    let Ok(registry) = context.try_service::<photag_contracts::mcp::McpToolRegistry>() else {
        return Ok(());
    };
    let tree_service = Arc::clone(tagging);
    let tag_tree = photag_contracts::mcp::McpToolDefinition {
        name: "tagging.list_tags".into(),
        title: "List hierarchical tags".into(),
        description: "Read PhoTag's hierarchical tag tree.".into(),
        input_schema:
            serde_json::json!({"type":"object","properties":{},"additionalProperties":false})
                .as_object()
                .cloned()
                .unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let tagging = Arc::clone(&tree_service);
            Box::pin(async move {
                let tags = tagging.tag_tree().map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"tags":tags}))
            })
        }),
    };
    let assign_service = Arc::clone(tagging);
    let assign = photag_contracts::mcp::McpToolDefinition {
        name: "tagging.assign".into(),
        title: "Assign tags".into(),
        description: "Assign tags to indexed media. Accepts 1 to 100 media IDs and 1 to 50 tag IDs.".into(),
        input_schema: serde_json::json!({
            "type":"object","properties":{
                "media_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":100},
                "tag_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":50}
            },"required":["media_ids","tag_ids"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let tagging = Arc::clone(&assign_service);
            Box::pin(async move {
                let request: TaggingMcpMutation = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                validate_tagging_mcp_ids(&request)?;
                tagging.assign(&request.media_ids, &request.tag_ids).map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"ok":true,"media_count":request.media_ids.len(),"tag_count":request.tag_ids.len()}))
            })
        }),
    };
    let remove_service = Arc::clone(tagging);
    let remove = photag_contracts::mcp::McpToolDefinition {
        name: "tagging.remove".into(),
        title: "Remove tags".into(),
        description: "Remove selected tags from indexed media. Accepts 1 to 100 media IDs and 1 to 50 tag IDs.".into(),
        input_schema: serde_json::json!({
            "type":"object","properties":{
                "media_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":100},
                "tag_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":50}
            },"required":["media_ids","tag_ids"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let tagging = Arc::clone(&remove_service);
            Box::pin(async move {
                let request: TaggingMcpMutation = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                validate_tagging_mcp_ids(&request)?;
                tagging.remove(&request.media_ids, &request.tag_ids).map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"ok":true,"media_count":request.media_ids.len(),"tag_count":request.tag_ids.len()}))
            })
        }),
    };
    let missing_service = Arc::clone(tagging);
    let missing = photag_contracts::mcp::McpToolDefinition {
        name: "tagging.list_missing_records".into(),
        title: "List missing tagged records".into(),
        description: "List indexed records with tags whose original files are missing; file contents are not read or removed.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{},"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |_| {
            let tagging = Arc::clone(&missing_service);
            Box::pin(async move {
                let records = tokio::task::spawn_blocking(move || tagging.missing_records())
                    .await.map_err(|error| error.to_string())??;
                Ok(serde_json::json!({"records":records}))
            })
        }),
    };
    let relink_service = Arc::clone(tagging);
    let relink_context = context.clone();
    let relink = photag_contracts::mcp::McpToolDefinition {
        name: "tagging.relink_missing_record".into(),
        title: "Relink missing tagged record".into(),
        description: "Associate one missing tagged record with an explicitly selected existing file; the target must be a regular file without conflicting tags or rating.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"record_id":{"type":"integer","minimum":1},"path":{"type":"string","minLength":1}},"required":["record_id","path"],"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let tagging = Arc::clone(&relink_service);
            let context = relink_context.clone();
            Box::pin(async move {
                let record_id = arguments.get("record_id").and_then(serde_json::Value::as_i64)
                    .filter(|id| *id > 0).ok_or_else(|| "record_id must be positive".to_owned())?;
                let path = arguments.get("path").and_then(serde_json::Value::as_str)
                    .filter(|path| !path.trim().is_empty()).ok_or_else(|| "path must be non-empty".to_owned())?.to_owned();
                tokio::task::spawn_blocking(move || tagging.relink_record(record_id, &path))
                    .await.map_err(|error| error.to_string())??;
                context.emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
                    .await.map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"relinked":record_id}))
            })
        }),
    };
    let delete_service = Arc::clone(tagging);
    let delete_context = context.clone();
    let delete_missing = photag_contracts::mcp::McpToolDefinition {
        name: "tagging.delete_missing_records".into(),
        title: "Delete missing record rows".into(),
        description: "Forget 1–500 indexed records only after verifying their local files are still missing. Never deletes a file from disk; requires explicit confirmation.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"record_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":500}},"required":["record_ids"],"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: true,
        handler: Arc::new(move |arguments| {
            let tagging = Arc::clone(&delete_service);
            let context = delete_context.clone();
            Box::pin(async move {
                let record_ids = arguments.get("record_ids").and_then(serde_json::Value::as_array)
                    .ok_or_else(|| "record_ids must be an array".to_owned())?.iter()
                    .map(|id| id.as_i64().filter(|id| *id > 0).ok_or_else(|| "record_ids must contain positive integers".to_owned()))
                    .collect::<Result<Vec<_>,_>>()?;
                if !(1..=500).contains(&record_ids.len()) { return Err("record_ids must contain 1 to 500 IDs".into()); }
                let count = tokio::task::spawn_blocking(move || tagging.delete_missing_records(&record_ids))
                    .await.map_err(|error| error.to_string())??;
                context.emit::<photag_contracts::MediaIndexChanged>(cordis::Routing::Unscoped, ())
                    .await.map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"deleted":count}))
            })
        }),
    };
    let registration = registry
        .register(
            "photag.tagging",
            [tag_tree, assign, remove, missing, relink, delete_missing],
        )
        .map_err(|error| TaggingPluginError::Publish(error.to_string()))?;
    let _mcp_registration = context
        .provide(Arc::new(TaggingMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| TaggingPluginError::Publish(error.to_string()))?;
    Ok(())
}

struct TaggingMcpRegistration {
    _registration: Arc<photag_contracts::mcp::McpToolRegistration>,
}

impl Service for TaggingMcpRegistration {
    const NAME: &'static str = "photag/tagging-mcp-registration";
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TaggingMcpMutation {
    media_ids: Vec<i64>,
    tag_ids: Vec<i64>,
}

fn validate_tagging_mcp_ids(request: &TaggingMcpMutation) -> Result<(), String> {
    if !(1..=100).contains(&request.media_ids.len()) || request.media_ids.iter().any(|id| *id <= 0)
    {
        return Err("media_ids must contain 1 to 100 positive IDs".into());
    }
    if !(1..=50).contains(&request.tag_ids.len()) || request.tag_ids.iter().any(|id| *id <= 0) {
        return Err("tag_ids must contain 1 to 50 positive IDs".into());
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum TaggingPluginError {
    #[error("required tagging dependency unavailable: {0}")]
    Dependency(String),
    #[error("tagging service publication failed: {0}")]
    Publish(String),
    #[error("tagging UI registration failed: {0}")]
    UiRegistration(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writeback::{TEST_LOCK, wait_until_idle};
    use photag_plugin_database::queries;
    use std::time::Duration;

    fn temp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "photag-tagging-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn assignment_preserves_associations_ancestors_and_manual_flags() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        assert!(wait_until_idle(Duration::from_secs(5)));
        let root = temp_root();
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let (media_id, parent_id, child_id, associated_id) = database
            .with_writer(|connection| {
                let folder = queries::add_folder(connection, "/tagging-test")?;
                let media_id = queries::insert_media(
                    connection,
                    folder.id,
                    "sample.jpg",
                    "/tagging-test/sample.jpg",
                    "image",
                    12,
                    1_700_000_000,
                    None,
                    None,
                    None,
                )?;
                let parent = queries::create_tag(connection, "Person", None)?;
                let child = queries::create_tag(connection, "Alex", Some(parent.id))?;
                let associated = queries::create_tag(connection, "Portrait", None)?;
                queries::set_associated_tags(connection, child.id, &[associated.id])?;
                Ok((media_id, parent.id, child.id, associated.id))
            })
            .unwrap();
        let tagging = TaggingService::new(Arc::clone(&database));

        tagging.assign(&[media_id], &[child_id]).unwrap();
        let assigned = tagging.for_media(media_id).unwrap();
        let ids: Vec<_> = assigned.iter().map(|tag| tag.id).collect();
        assert!(ids.contains(&parent_id));
        assert!(ids.contains(&child_id));
        assert!(ids.contains(&associated_id));
        let flags: std::collections::HashMap<_, _> = database
            .with_reader(|connection| {
                let mut statement = connection
                    .prepare("SELECT tag_id, auto_applied FROM media_tags WHERE media_id = ?1")?;
                let rows = statement.query_map([media_id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? != 0))
                })?;
                rows.collect()
            })
            .unwrap();
        assert_eq!(flags.get(&child_id), Some(&false));
        assert_eq!(flags.get(&associated_id), Some(&false));
        assert_eq!(flags.get(&parent_id), Some(&true));

        tagging.remove(&[media_id], &[child_id]).unwrap();
        assert!(
            !tagging
                .for_media(media_id)
                .unwrap()
                .iter()
                .any(|tag| tag.id == child_id)
        );
        tagging.clear(&[media_id]).unwrap();
        assert!(tagging.for_media(media_id).unwrap().is_empty());
        assert!(wait_until_idle(Duration::from_secs(5)));
        let _ = std::fs::remove_dir_all(root);
    }
}
