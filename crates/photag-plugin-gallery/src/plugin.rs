//! Cordis lifecycle for the gallery, including local browser providers.
use super::*;

#[derive(Debug, Clone, Default)]
pub struct GalleryConfig;

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    type Config = GalleryConfig;
    type Input = GalleryConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = GalleryPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.gallery".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let file_operations = context
            .try_service::<FileOperationsService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let drag_drop = context
            .try_service::<DragDropService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let quicklook = context
            .try_service::<QuickLookService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let session = context
            .try_service::<SessionService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        let versions = context
            .try_service::<photag_plugin_versions::VersionsService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        // History is optional so Gallery remains usable when the development
        // history plugin is not part of the active Cordis graph.
        let history = context
            .try_service::<photag_plugin_history::HistoryService>()
            .ok();
        let settings = context
            .try_service::<photag_plugin_settings::SettingsService>()
            .ok();
        let tagging = context
            .try_service::<photag_plugin_tagging::TaggingService>()
            .ok();
        let runtime = tokio::runtime::Handle::current();
        let mut service = GalleryService::new(database);
        service.grouping = Some(
            context
                .try_service::<photag_plugin_grouping::GroupingService>()
                .map_err(|e| GalleryPluginError::Dependency(e.to_string()))?,
        );
        service.indexing = context
            .try_service::<photag_plugin_indexing::IndexingService>()
            .ok();
        service.watcher = context
            .try_service::<photag_plugin_watcher::WatcherService>()
            .ok();
        let service = Arc::new(service);
        let selected_folder = Arc::new(SelectedFolder::default());
        let selected_tag = Arc::new(SelectedTag::default());
        let selected_path = Arc::new(SelectedPath::default());
        let index_revision = Arc::new(IndexRevision::default());
        let _selection_publication = context
            .provide(Arc::clone(&selected_folder))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let _tag_selection_publication = context
            .provide(Arc::clone(&selected_tag))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let _path_selection_publication = context
            .provide(Arc::clone(&selected_path))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let _index_revision_publication = context
            .provide(Arc::clone(&index_revision))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        let selection_state = Arc::clone(&selected_folder);
        let selection_tag_state = Arc::clone(&selected_tag);
        let selection_path_state = Arc::clone(&selected_path);
        let ready = service.wake.clone();
        let inspector_selection = Arc::clone(&service);
        let _selection_listener = context
            .on::<FolderSelectionChanged, _>(observer_sync(move |_, folder_id| {
                if let Ok(mut selected) = selection_state.0.lock() {
                    *selected = folder_id;
                }
                if let Ok(mut selected) = selection_tag_state.0.lock() {
                    *selected = TagFilterSelection::default();
                }
                if let Ok(mut selected) = selection_path_state.0.lock() {
                    *selected = None;
                }
                inspector_selection
                    .navigation_revision
                    .fetch_add(1, Ordering::Release);
                ready.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let tag_state = Arc::clone(&selected_tag);
        let tag_folder_state = Arc::clone(&selected_folder);
        let tag_path_state = Arc::clone(&selected_path);
        let ready = service.wake.clone();
        let inspector_selection = Arc::clone(&service);
        let _tag_selection_listener = context
            .on::<TagSelectionChanged, _>(observer_sync(move |_, tag_id: Option<i64>| {
                if let Ok(mut selected) = tag_state.0.lock() {
                    *selected = TagFilterSelection {
                        included: tag_id.into_iter().collect(),
                        excluded: Vec::new(),
                    };
                }
                if tag_id.is_some() {
                    if let Ok(mut selected) = tag_folder_state.0.lock() {
                        *selected = None;
                    }
                    if let Ok(mut selected) = tag_path_state.0.lock() {
                        *selected = None;
                    }
                }
                inspector_selection
                    .navigation_revision
                    .fetch_add(1, Ordering::Release);
                ready.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let tag_filter_state = Arc::clone(&selected_tag);
        let tag_filter_folder = Arc::clone(&selected_folder);
        let tag_filter_path = Arc::clone(&selected_path);
        let ready = service.wake.clone();
        let inspector_selection = Arc::clone(&service);
        let _tag_filter_selection_listener = context
            .on::<TagFilterSelectionChanged, _>(observer_sync(
                move |_, selection: TagFilterSelection| {
                    if let Ok(mut selected) = tag_filter_state.0.lock() {
                        *selected = selection.clone();
                    }
                    if !selection.included.is_empty() || !selection.excluded.is_empty() {
                        if let Ok(mut selected) = tag_filter_folder.0.lock() {
                            *selected = None;
                        }
                        if let Ok(mut selected) = tag_filter_path.0.lock() {
                            *selected = None;
                        }
                    }
                    inspector_selection
                        .navigation_revision
                        .fetch_add(1, Ordering::Release);
                    ready.trigger();
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let path_state = Arc::clone(&selected_path);
        let path_folder_state = Arc::clone(&selected_folder);
        let path_tag_state = Arc::clone(&selected_tag);
        let ready = service.wake.clone();
        let inspector_selection = Arc::clone(&service);
        let _path_selection_listener = context
            .on::<DirectoryPathSelected, _>(observer_sync(move |_, path| {
                if let Ok(mut selected) = path_state.0.lock() {
                    *selected = path;
                }
                if let Ok(mut selected) = path_folder_state.0.lock() {
                    *selected = None;
                }
                if let Ok(mut selected) = path_tag_state.0.lock() {
                    *selected = TagFilterSelection::default();
                }
                inspector_selection
                    .navigation_revision
                    .fetch_add(1, Ordering::Release);
                ready.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let revision = Arc::clone(&index_revision);
        let ready = service.wake.clone();
        let _media_index_listener = context
            .on::<MediaIndexChanged, _>(observer_sync(move |_, ()| {
                revision.0.fetch_add(1, Ordering::Release);
                ready.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        register_for_plugin(
            &context,
            "photag.gallery",
            [
                UiContribution::new(
                    "gallery/workspace",
                    ContributionKind::Layout {
                        label: "PhoTag workspace".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "gallery/main-slot",
                    ContributionKind::Slot {
                        label: "gallery.content".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "gallery/grid",
                    ContributionKind::Widget {
                        label: "Media Gallery".into(),
                        slot: "gallery.content".into(),
                        order: 0,
                    },
                ),
            ],
        )
        .map_err(|error| GalleryPluginError::UiRegistration(error.to_string()))?;
        let bridge = context
            .try_service::<photag_ui_bridge::UiBridgeService>()
            .map_err(|error| GalleryPluginError::Dependency(error.to_string()))?;
        if let (Ok(ai), Some(settings), Some(tagging), Some(history)) = (
            context.try_service::<photag_plugin_ai::AiService>(),
            settings.as_ref(),
            tagging.as_ref(),
            history.as_ref(),
        ) {
            register_views_for_plugin(
                &context,
                "photag.gallery",
                [ai_suggestions::view(
                    Arc::clone(&service.ai_target),
                    ai,
                    Arc::clone(settings),
                    Arc::clone(tagging),
                    Arc::clone(history),
                )],
            )
            .map_err(|error| GalleryPluginError::UiRegistration(error.to_string()))?;
        }
        register_views_for_plugin(
            &context,
            "photag.gallery",
            view::gallery(
                service,
                file_operations,
                drag_drop,
                quicklook,
                versions,
                runtime,
                context.clone(),
                selected_folder,
                selected_tag,
                selected_path,
                index_revision,
                session,
                history,
                settings,
                tagging,
                Arc::clone(&bridge.surfaces),
                Arc::clone(&bridge.views),
            ),
        )
        .map_err(|error| GalleryPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GalleryPluginError {
    #[error("required gallery dependency unavailable: {0}")]
    Dependency(String),
    #[error("gallery service publication failed: {0}")]
    Publish(String),
    #[error("gallery UI contribution registration failed: {0}")]
    UiRegistration(String),
    #[error("gallery event registration failed: {0}")]
    EventRegistration(String),
}
