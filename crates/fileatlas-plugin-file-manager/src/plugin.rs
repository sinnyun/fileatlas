//! Cordis lifecycle for the file_manager, including local browser providers.
use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct McpFileManagerSearchRequest {
    search: Option<String>,
    folder_id: Option<i64>,
    tag_ids: Option<Vec<i64>>,
    media_type: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
    sort: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FileManagerConfig;

pub struct FileManagerPlugin;

impl Plugin for FileManagerPlugin {
    type Config = FileManagerConfig;
    type Input = FileManagerConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = FileManagerPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        "fileatlas.file_manager".into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let file_operations = context
            .try_service::<FileOperationsService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let drag_drop = context
            .try_service::<DragDropService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let quicklook = context
            .try_service::<QuickLookService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let session = context
            .try_service::<SessionService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let workspaces = context
            .try_service::<fileatlas_plugin_workspaces::WorkspaceService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let versions = context
            .try_service::<fileatlas_plugin_versions::VersionsService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        // History is optional so FileManager remains usable when the development
        // history plugin is not part of the active Cordis graph.
        let history = context
            .try_service::<fileatlas_plugin_history::HistoryService>()
            .ok();
        let settings = context
            .try_service::<fileatlas_plugin_settings::SettingsService>()
            .ok();
        let tagging = context
            .try_service::<fileatlas_plugin_tagging::TaggingService>()
            .ok();
        let runtime = tokio::runtime::Handle::current();
        let mut service = FileManagerService::new(database);
        service.grouping = Some(
            context
                .try_service::<fileatlas_plugin_grouping::GroupingService>()
                .map_err(|e| FileManagerPluginError::Dependency(e.to_string()))?,
        );
        service.indexing = context
            .try_service::<fileatlas_plugin_indexing::IndexingService>()
            .ok();
        service.everything = context
            .try_service::<fileatlas_plugin_everything_search::EverythingSearchService>()
            .ok();
        service.watcher = context
            .try_service::<fileatlas_plugin_watcher::WatcherService>()
            .ok();
        let service = Arc::new(service);
        let columns = context
            .try_service::<fileatlas_plugin_columns::ColumnLayoutService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        let column_services: [Arc<FileManagerService>; 4] = std::array::from_fn(|index| {
            if index == 0 {
                Arc::clone(&service)
            } else {
                Arc::new(service.fork_for_column())
            }
        });
        let selected_folders: [Arc<SelectedFolder>; 4] =
            std::array::from_fn(|_| Arc::new(SelectedFolder::default()));
        let selected_tags: [Arc<SelectedTag>; 4] =
            std::array::from_fn(|_| Arc::new(SelectedTag::default()));
        let selected_paths: [Arc<SelectedPath>; 4] =
            std::array::from_fn(|_| Arc::new(SelectedPath::default()));
        let index_revision = Arc::new(IndexRevision::default());
        let _selection_publication = context
            .provide(Arc::clone(&selected_folders[0]))
            .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
        let _tag_selection_publication = context
            .provide(Arc::clone(&selected_tags[0]))
            .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
        let _path_selection_publication = context
            .provide(Arc::clone(&selected_paths[0]))
            .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
        let _index_revision_publication = context
            .provide(Arc::clone(&index_revision))
            .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
        let selection_states = selected_folders.clone();
        let selection_tag_states = selected_tags.clone();
        let selection_path_states = selected_paths.clone();
        let selection_services = column_services.clone();
        let selection_columns = Arc::clone(&columns);
        let _selection_listener = context
            .on::<FolderSelectionChanged, _>(observer_sync(move |_, folder_id| {
                let index = selection_columns
                    .snapshot()
                    .map(|state| state.active_index.min(3))
                    .unwrap_or(0);
                if let Ok(mut selected) = selection_states[index].0.lock() {
                    *selected = folder_id;
                }
                if let Ok(mut selected) = selection_tag_states[index].0.lock() {
                    *selected = TagFilterSelection::default();
                }
                if let Ok(mut selected) = selection_path_states[index].0.lock() {
                    *selected = None;
                }
                let target = &selection_services[index];
                target.navigation_revision.fetch_add(1, Ordering::Release);
                target.wake.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| FileManagerPluginError::EventRegistration(error.to_string()))?;
        let tag_states = selected_tags.clone();
        let tag_folder_states = selected_folders.clone();
        let tag_path_states = selected_paths.clone();
        let tag_services = column_services.clone();
        let tag_columns = Arc::clone(&columns);
        let _tag_selection_listener = context
            .on::<TagSelectionChanged, _>(observer_sync(move |_, tag_id: Option<i64>| {
                let index = tag_columns
                    .snapshot()
                    .map(|state| state.active_index.min(3))
                    .unwrap_or(0);
                if let Ok(mut selected) = tag_states[index].0.lock() {
                    *selected = TagFilterSelection {
                        included: tag_id.into_iter().collect(),
                        excluded: Vec::new(),
                    };
                }
                if tag_id.is_some() {
                    if let Ok(mut selected) = tag_folder_states[index].0.lock() {
                        *selected = None;
                    }
                    if let Ok(mut selected) = tag_path_states[index].0.lock() {
                        *selected = None;
                    }
                }
                let target = &tag_services[index];
                target.navigation_revision.fetch_add(1, Ordering::Release);
                target.wake.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| FileManagerPluginError::EventRegistration(error.to_string()))?;
        let tag_filter_states = selected_tags.clone();
        let tag_filter_folders = selected_folders.clone();
        let tag_filter_paths = selected_paths.clone();
        let tag_filter_services = column_services.clone();
        let tag_filter_columns = Arc::clone(&columns);
        let _tag_filter_selection_listener = context
            .on::<TagFilterSelectionChanged, _>(observer_sync(
                move |_, selection: TagFilterSelection| {
                    let index = tag_filter_columns
                        .snapshot()
                        .map(|state| state.active_index.min(3))
                        .unwrap_or(0);
                    if let Ok(mut selected) = tag_filter_states[index].0.lock() {
                        *selected = selection.clone();
                    }
                    if !selection.included.is_empty() || !selection.excluded.is_empty() {
                        if let Ok(mut selected) = tag_filter_folders[index].0.lock() {
                            *selected = None;
                        }
                        if let Ok(mut selected) = tag_filter_paths[index].0.lock() {
                            *selected = None;
                        }
                    }
                    let target = &tag_filter_services[index];
                    target.navigation_revision.fetch_add(1, Ordering::Release);
                    target.wake.trigger();
                    Ok::<_, std::convert::Infallible>(())
                },
            ))
            .map_err(|error| FileManagerPluginError::EventRegistration(error.to_string()))?;
        let path_states = selected_paths.clone();
        let path_folder_states = selected_folders.clone();
        let path_tag_states = selected_tags.clone();
        let path_services = column_services.clone();
        let path_columns = Arc::clone(&columns);
        let _path_selection_listener = context
            .on::<DirectoryPathSelected, _>(observer_sync(move |_, path| {
                let index = path_columns
                    .snapshot()
                    .map(|state| state.active_index.min(3))
                    .unwrap_or(0);
                if let Ok(mut selected) = path_states[index].0.lock() {
                    *selected = path;
                }
                if let Ok(mut selected) = path_folder_states[index].0.lock() {
                    *selected = None;
                }
                if let Ok(mut selected) = path_tag_states[index].0.lock() {
                    *selected = TagFilterSelection::default();
                }
                let target = &path_services[index];
                target.navigation_revision.fetch_add(1, Ordering::Release);
                target.wake.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| FileManagerPluginError::EventRegistration(error.to_string()))?;
        let revision = Arc::clone(&index_revision);
        let ready = service.wake.clone();
        let _media_index_listener = context
            .on::<MediaIndexChanged, _>(observer_sync(move |_, ()| {
                revision.0.fetch_add(1, Ordering::Release);
                ready.trigger();
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| FileManagerPluginError::EventRegistration(error.to_string()))?;
        let version_file_manager = Arc::clone(&service);
        let _version_history_listener = context
            .on::<MediaVersionHistoryChanged, _>(observer_sync(move |_, media_id| {
                version_file_manager.request_ui(FileManagerUiRequest::HistoryChanged(media_id));
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| FileManagerPluginError::EventRegistration(error.to_string()))?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
        register_file_manager_mcp_tools(&context, &service)?;
        register_for_plugin(
            &context,
            "fileatlas.file_manager",
            [
                UiContribution::new(
                    "file_manager/workspace",
                    ContributionKind::Layout {
                        label: "FileAtlas workspace".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "file_manager/main-slot",
                    ContributionKind::Slot {
                        label: "file_manager.content".into(),
                        order: 0,
                    },
                ),
                UiContribution::new(
                    "file_manager/grid",
                    ContributionKind::Widget {
                        label: "File manager".into(),
                        slot: "file_manager.content".into(),
                        order: 0,
                    },
                ),
            ],
        )
        .map_err(|error| FileManagerPluginError::UiRegistration(error.to_string()))?;
        let bridge = context
            .try_service::<fileatlas_ui_bridge::UiBridgeService>()
            .map_err(|error| FileManagerPluginError::Dependency(error.to_string()))?;
        if let (Ok(ai), Some(settings), Some(tagging), Some(history)) = (
            context.try_service::<fileatlas_plugin_ai::AiService>(),
            settings.as_ref(),
            tagging.as_ref(),
            history.as_ref(),
        ) {
            register_views_for_plugin(
                &context,
                "fileatlas.file_manager",
                [ai_suggestions::view(
                    Arc::clone(&service.ai_target),
                    ai,
                    Arc::clone(settings),
                    Arc::clone(tagging),
                    Arc::clone(history),
                )],
            )
            .map_err(|error| FileManagerPluginError::UiRegistration(error.to_string()))?;
        }
        register_views_for_plugin(
            &context,
            "fileatlas.file_manager",
            view::file_manager(
                column_services,
                file_operations,
                drag_drop,
                quicklook,
                versions,
                runtime,
                context.clone(),
                selected_folders,
                selected_tags,
                selected_paths,
                index_revision,
                session,
                history,
                settings,
                tagging,
                Arc::clone(&bridge.surfaces),
                Arc::clone(&bridge.views),
                workspaces,
            ),
        )
        .map_err(|error| FileManagerPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

fn register_file_manager_mcp_tools(
    context: &Context,
    file_manager: &Arc<FileManagerService>,
) -> Result<(), FileManagerPluginError> {
    let Ok(registry) = context.try_service::<fileatlas_contracts::mcp::McpToolRegistry>() else {
        return Ok(());
    };
    let search_file_manager = Arc::clone(file_manager);
    let search = fileatlas_contracts::mcp::McpToolDefinition {
        name: "file_manager.search_media".into(),
        title: "Search files".into(),
        description: "Search indexed files with folder, tag, media type, paging, and sort filters.".into(),
        input_schema: serde_json::json!({
            "type":"object",
            "properties":{
                "search":{"type":"string"},
                "folder_id":{"type":"integer"},
                "tag_ids":{"type":"array","items":{"type":"integer"},"maxItems":50},
                "media_type":{"type":"string","enum":["image","video","audio","file"]},
                "offset":{"type":"integer","minimum":0,"maximum":1000000},
                "limit":{"type":"integer","minimum":1,"maximum":100},
                "sort":{"type":"string","enum":["newest","oldest","largest","smallest","name_asc","name_desc","rated","unrated_first"]}
            },"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let file_manager = Arc::clone(&search_file_manager);
            Box::pin(async move {
                let request: McpFileManagerSearchRequest = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                let offset = request.offset.unwrap_or(0);
                let limit = request.limit.unwrap_or(50);
                if offset > 1_000_000 || !(1..=100).contains(&limit) {
                    return Err("offset must be 0..=1000000 and limit must be 1..=100".into());
                }
                if request.tag_ids.as_ref().is_some_and(|ids| ids.len() > 50) {
                    return Err("at most 50 tag IDs are allowed".into());
                }
                let media_type = request.media_type;
                if media_type.as_ref().is_some_and(|value| !["image", "video", "audio", "file"].contains(&value.as_str())) {
                    return Err("media_type must be image, video, audio, or file".into());
                }
                let sort = match request.sort.as_deref().unwrap_or("newest") {
                    "newest" => Some(fileatlas_plugin_database::SortOrder::NewestFirst),
                    "oldest" => Some(fileatlas_plugin_database::SortOrder::OldestFirst),
                    "largest" => Some(fileatlas_plugin_database::SortOrder::BiggestFirst),
                    "smallest" => Some(fileatlas_plugin_database::SortOrder::SmallestFirst),
                    "name_asc" => Some(fileatlas_plugin_database::SortOrder::NameAsc),
                    "name_desc" => Some(fileatlas_plugin_database::SortOrder::NameDesc),
                    "rated" => Some(fileatlas_plugin_database::SortOrder::HighestRated),
                    "unrated_first" => Some(fileatlas_plugin_database::SortOrder::LowestRated),
                    _ => return Err("unsupported sort order".into()),
                };
                let search_term = request.search;
                let query = fileatlas_plugin_database::MediaQuery {
                    folder_id: request.folder_id,
                    tag_ids: request.tag_ids,
                    search_in_filenames: search_term.is_some(),
                    search_in_paths: search_term.is_some(),
                    search_in_tags: search_term.is_some(),
                    search: search_term,
                    media_type,
                    sort,
                    ..Default::default()
                };
                let snapshot = file_manager.load(query.clone()).map_err(|error| error.to_string())?;
                let provider = snapshot.search_provider;
                let items = if offset == 0 {
                    snapshot.media.into_iter().take(limit).collect::<Vec<_>>()
                } else {
                    file_manager.load_page(query, offset).map_err(|error| error.to_string())?
                        .media.into_iter().take(limit).collect::<Vec<_>>()
                };
                Ok(serde_json::json!({"total":snapshot.media_count,"offset":offset,"items":items,"search_provider":provider}))
            })
        }),
    };
    let get_file_manager = Arc::clone(file_manager);
    let get_media = fileatlas_contracts::mcp::McpToolDefinition {
        name: "file_manager.get_media".into(),
        title: "Read media details".into(),
        description: "Read one indexed media record and the FileAtlas tags assigned to it.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1}},"required":["media_id"],"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let file_manager = Arc::clone(&get_file_manager);
            Box::pin(async move {
                let id = arguments.get("media_id").and_then(serde_json::Value::as_i64)
                    .filter(|id| *id > 0).ok_or_else(|| "media_id must be a positive integer".to_owned())?;
                let media = file_manager.media_by_id(id).map_err(|error| error.to_string())?
                    .ok_or_else(|| "media record was not found".to_owned())?;
                let tags = file_manager.tags_for_media(&[id]).map_err(|error| error.to_string())?
                    .remove(&id).unwrap_or_default();
                Ok(serde_json::json!({"media":media,"tags":tags}))
            })
        }),
    };
    let rating_file_manager = Arc::clone(file_manager);
    let set_rating = fileatlas_contracts::mcp::McpToolDefinition {
        name: "file_manager.set_rating".into(),
        title: "Set media rating".into(),
        description: "Set a 0–5 star rating on 1 to 100 indexed media records.".into(),
        input_schema: serde_json::json!({
            "type":"object","properties":{
                "media_ids":{"type":"array","items":{"type":"integer","minimum":1},"minItems":1,"maxItems":100},
                "rating":{"type":"integer","minimum":0,"maximum":5}
            },"required":["media_ids","rating"],"additionalProperties":false
        }).as_object().cloned().unwrap_or_default(),
        mutating: true,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let file_manager = Arc::clone(&rating_file_manager);
            Box::pin(async move {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Request { media_ids: Vec<i64>, rating: u8 }
                let request: Request = serde_json::from_value(arguments)
                    .map_err(|error| format!("invalid parameters: {error}"))?;
                if !(1..=100).contains(&request.media_ids.len())
                    || request.media_ids.iter().any(|id| *id <= 0)
                {
                    return Err("media_ids must contain 1 to 100 positive IDs".into());
                }
                if request.rating > 5 {
                    return Err("rating must be between 0 and 5".into());
                }
                file_manager.apply_rating(&request.media_ids, request.rating)
                    .map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"ok":true,"count":request.media_ids.len(),"rating":request.rating}))
            })
        }),
    };
    let registration = registry
        .register("fileatlas.file_manager", [search, get_media, set_rating])
        .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
    let _mcp_registration = context
        .provide(Arc::new(FileManagerMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| FileManagerPluginError::Publish(error.to_string()))?;
    Ok(())
}

struct FileManagerMcpRegistration {
    _registration: Arc<fileatlas_contracts::mcp::McpToolRegistration>,
}

impl cordis::Service for FileManagerMcpRegistration {
    const NAME: &'static str = "fileatlas/file_manager-mcp-registration";
}

#[derive(Debug, thiserror::Error)]
pub enum FileManagerPluginError {
    #[error("required file manager dependency unavailable: {0}")]
    Dependency(String),
    #[error("file manager service publication failed: {0}")]
    Publish(String),
    #[error("file manager UI contribution registration failed: {0}")]
    UiRegistration(String),
    #[error("file manager event registration failed: {0}")]
    EventRegistration(String),
}
