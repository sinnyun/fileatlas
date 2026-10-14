//! PhoTag media-tag assignments and tag associations, owned by Cordis.

use std::sync::{Arc, Mutex};

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::db::queries::{self, Tag};
use photag_ui_bridge::{ContributionKind, UiContribution, register_for_plugin};

pub mod writeback;
pub mod xmp;

pub struct TaggingService {
    database: Arc<DatabaseService>,
    recent_tag_ids: Mutex<Vec<i64>>,
}

impl Service for TaggingService {
    const NAME: &'static str = "photag/tagging";
}

impl TaggingService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self {
            database,
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
        self.database
            .with_writer(|connection| queries::assign_tags(connection, media_ids, tag_ids))?;
        if track_recent && let Ok(mut recent) = self.recent_tag_ids.lock() {
            for &tag_id in tag_ids.iter().rev() {
                recent.retain(|id| *id != tag_id);
                recent.insert(0, tag_id);
            }
            recent.truncate(8);
        }
        self.schedule_xmp_sync(media_ids);
        Ok(())
    }

    /// Recently assigned tag IDs, newest first, shared by Gallery and Preview.
    pub fn recent_tags(&self) -> Vec<i64> {
        self.recent_tag_ids
            .lock()
            .map(|recent| recent.clone())
            .unwrap_or_default()
    }

    #[tracing::instrument(skip_all, err)]
    pub fn remove(&self, media_ids: &[i64], tag_ids: &[i64]) -> Result<(), TaggingError> {
        self.database
            .with_writer(|connection| queries::remove_tags(connection, media_ids, tag_ids))?;
        self.schedule_xmp_sync(media_ids);
        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    pub fn clear(&self, media_ids: &[i64]) -> Result<(), TaggingError> {
        self.database
            .with_writer(|connection| queries::remove_all_tags(connection, media_ids))?;
        self.schedule_xmp_sync(media_ids);
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
        let tag_ids = self.database.with_writer(|connection| {
            let parent = queries::find_or_create_tag(connection, "AI Suggested");
            let mut ids = std::collections::BTreeSet::new();
            for label in labels {
                if let Some(id) = queries::find_tag_by_name(connection, label) {
                    ids.insert(id);
                } else if let Ok(tag) = queries::create_tag(connection, label, parent) {
                    ids.insert(tag.id);
                }
            }
            Ok::<_, rusqlite::Error>(ids.into_iter().collect::<Vec<_>>())
        })?;
        self.assign(&[media_id], &tag_ids)?;
        Ok(tag_ids)
    }

    pub fn for_media(&self, media_id: i64) -> Result<Vec<Tag>, TaggingError> {
        Ok(self
            .database
            .with_reader(|connection| queries::get_media_tags(connection, media_id))?)
    }

    /// Return the current hierarchical tag tree for assignment pickers.
    pub fn tag_tree(&self) -> Result<Vec<Tag>, TaggingError> {
        Ok(self.database.with_reader(queries::get_tag_tree)?)
    }

    pub fn associated_with(&self, tag_id: i64) -> Result<Vec<i64>, TaggingError> {
        Ok(self
            .database
            .with_reader(|connection| queries::get_associated_tags(connection, tag_id))?)
    }

    #[tracing::instrument(skip_all, err)]
    pub fn set_associations(
        &self,
        tag_id: i64,
        associated_ids: &[i64],
    ) -> Result<(), TaggingError> {
        Ok(self.database.with_writer(|connection| {
            queries::set_associated_tags(connection, tag_id, associated_ids)
        })?)
    }

    #[tracing::instrument(skip_all, err)]
    pub fn merge(
        &self,
        source_id: i64,
        target_id: i64,
        reparent_children: bool,
    ) -> Result<(), TaggingError> {
        let affected = self
            .database
            .with_reader(|connection| queries::get_media_ids_with_tag(connection, source_id))?;
        self.database.with_writer(|connection| {
            queries::merge_tag(connection, source_id, target_id, reparent_children)
        })?;
        self.schedule_xmp_sync(&affected);
        Ok(())
    }

    /// Reparent a tag and let the query report media whose implicit ancestor
    /// assignments changed; those media receive a fresh metadata write.
    #[tracing::instrument(skip_all, err)]
    pub fn move_tag(
        &self,
        source_id: i64,
        drop_position: queries::TagDropPos,
    ) -> Result<Vec<i64>, TaggingError> {
        let affected = self
            .database
            .with_writer(|connection| queries::move_tag(connection, source_id, drop_position))?;
        self.schedule_xmp_sync(&affected);
        Ok(affected)
    }

    /// Rewrite IPTC or XMP tags after a tag mutation. File writes run on a
    /// detached worker, matching PhoTag's best-effort synchronization policy.
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
        let _publication = context
            .provide(Arc::new(TaggingService::new(database)))
            .map_err(|error| TaggingPluginError::Publish(error.to_string()))?;
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
    use photag_plugin_database::db::queries;
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
