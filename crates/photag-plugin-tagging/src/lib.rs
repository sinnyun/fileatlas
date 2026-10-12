//! PhoTag media-tag assignments and tag associations, owned by Cordis.

use std::sync::Arc;

use cordis::{Context, Plugin, Service};
use photag_plugin_database::DatabaseService;
use photag_plugin_database::db::queries::{self, Tag};
use photag_ui_bridge::{ContributionKind, UiContribution, register_for_plugin};

pub struct TaggingService {
    database: Arc<DatabaseService>,
}

impl Service for TaggingService {
    const NAME: &'static str = "photag/tagging";
}

impl TaggingService {
    pub fn new(database: Arc<DatabaseService>) -> Self {
        Self { database }
    }

    /// Assign source tags, their one-way associated tags, and required ancestors.
    /// The database query preserves PhoTag's manual-versus-auto-applied flags.
    pub fn assign(&self, media_ids: &[i64], tag_ids: &[i64]) -> Result<(), TaggingError> {
        Ok(self
            .database
            .with_writer(|connection| queries::assign_tags(connection, media_ids, tag_ids))?)
    }

    pub fn remove(&self, media_ids: &[i64], tag_ids: &[i64]) -> Result<(), TaggingError> {
        Ok(self
            .database
            .with_writer(|connection| queries::remove_tags(connection, media_ids, tag_ids))?)
    }

    pub fn clear(&self, media_ids: &[i64]) -> Result<(), TaggingError> {
        Ok(self
            .database
            .with_writer(|connection| queries::remove_all_tags(connection, media_ids))?)
    }

    pub fn for_media(&self, media_id: i64) -> Result<Vec<Tag>, TaggingError> {
        Ok(self
            .database
            .with_reader(|connection| queries::get_media_tags(connection, media_id))?)
    }

    pub fn associated_with(&self, tag_id: i64) -> Result<Vec<i64>, TaggingError> {
        Ok(self
            .database
            .with_reader(|connection| queries::get_associated_tags(connection, tag_id))?)
    }

    pub fn set_associations(
        &self,
        tag_id: i64,
        associated_ids: &[i64],
    ) -> Result<(), TaggingError> {
        Ok(self.database.with_writer(|connection| {
            queries::set_associated_tags(connection, tag_id, associated_ids)
        })?)
    }

    pub fn merge(
        &self,
        source_id: i64,
        target_id: i64,
        reparent_children: bool,
    ) -> Result<(), TaggingError> {
        Ok(self.database.with_writer(|connection| {
            queries::merge_tag(connection, source_id, target_id, reparent_children)
        })?)
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
    use photag_plugin_database::db::queries;

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
        let _ = std::fs::remove_dir_all(root);
    }
}
