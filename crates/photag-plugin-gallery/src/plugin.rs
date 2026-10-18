//! Cordis lifecycle for the gallery, including local browser providers.
use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct McpGallerySearchRequest {
    search: Option<String>,
    folder_id: Option<i64>,
    tag_ids: Option<Vec<i64>>,
    media_type: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
    sort: Option<String>,
}

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
        service.everything = context
            .try_service::<photag_plugin_everything_search::EverythingSearchService>()
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
        let version_gallery = Arc::clone(&service);
        let _version_history_listener = context
            .on::<MediaVersionHistoryChanged, _>(observer_sync(move |_, media_id| {
                version_gallery.request_ui(GalleryUiRequest::HistoryChanged(media_id));
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|error| GalleryPluginError::EventRegistration(error.to_string()))?;
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
        register_gallery_mcp_tools(&context, &service)?;
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

fn register_gallery_mcp_tools(
    context: &Context,
    gallery: &Arc<GalleryService>,
) -> Result<(), GalleryPluginError> {
    let Ok(registry) = context.try_service::<photag_contracts::mcp::McpToolRegistry>() else {
        return Ok(());
    };
    let search_gallery = Arc::clone(gallery);
    let search = photag_contracts::mcp::McpToolDefinition {
        name: "gallery.search_media".into(),
        title: "Search gallery media".into(),
        description: "Search indexed gallery media with folder, tag, media type, paging, and sort filters.".into(),
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
            let gallery = Arc::clone(&search_gallery);
            Box::pin(async move {
                let request: McpGallerySearchRequest = serde_json::from_value(arguments)
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
                    "newest" => Some(photag_plugin_database::SortOrder::NewestFirst),
                    "oldest" => Some(photag_plugin_database::SortOrder::OldestFirst),
                    "largest" => Some(photag_plugin_database::SortOrder::BiggestFirst),
                    "smallest" => Some(photag_plugin_database::SortOrder::SmallestFirst),
                    "name_asc" => Some(photag_plugin_database::SortOrder::NameAsc),
                    "name_desc" => Some(photag_plugin_database::SortOrder::NameDesc),
                    "rated" => Some(photag_plugin_database::SortOrder::HighestRated),
                    "unrated_first" => Some(photag_plugin_database::SortOrder::LowestRated),
                    _ => return Err("unsupported sort order".into()),
                };
                let search_term = request.search;
                let query = photag_plugin_database::MediaQuery {
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
                let snapshot = gallery.load(query.clone()).map_err(|error| error.to_string())?;
                let provider = snapshot.search_provider;
                let items = if offset == 0 {
                    snapshot.media.into_iter().take(limit).collect::<Vec<_>>()
                } else {
                    gallery.load_page(query, offset).map_err(|error| error.to_string())?
                        .media.into_iter().take(limit).collect::<Vec<_>>()
                };
                Ok(serde_json::json!({"total":snapshot.media_count,"offset":offset,"items":items,"search_provider":provider}))
            })
        }),
    };
    let get_gallery = Arc::clone(gallery);
    let get_media = photag_contracts::mcp::McpToolDefinition {
        name: "gallery.get_media".into(),
        title: "Read media details".into(),
        description: "Read one indexed media record and the PhoTag tags assigned to it.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"media_id":{"type":"integer","minimum":1}},"required":["media_id"],"additionalProperties":false})
            .as_object().cloned().unwrap_or_default(),
        mutating: false,
        confirmation_required: false,
        handler: Arc::new(move |arguments| {
            let gallery = Arc::clone(&get_gallery);
            Box::pin(async move {
                let id = arguments.get("media_id").and_then(serde_json::Value::as_i64)
                    .filter(|id| *id > 0).ok_or_else(|| "media_id must be a positive integer".to_owned())?;
                let media = gallery.media_by_id(id).map_err(|error| error.to_string())?
                    .ok_or_else(|| "media record was not found".to_owned())?;
                let tags = gallery.tags_for_media(&[id]).map_err(|error| error.to_string())?
                    .remove(&id).unwrap_or_default();
                Ok(serde_json::json!({"media":media,"tags":tags}))
            })
        }),
    };
    let rating_gallery = Arc::clone(gallery);
    let set_rating = photag_contracts::mcp::McpToolDefinition {
        name: "gallery.set_rating".into(),
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
            let gallery = Arc::clone(&rating_gallery);
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
                gallery.apply_rating(&request.media_ids, request.rating)
                    .map_err(|error| error.to_string())?;
                Ok(serde_json::json!({"ok":true,"count":request.media_ids.len(),"rating":request.rating}))
            })
        }),
    };
    let registration = registry
        .register("photag.gallery", [search, get_media, set_rating])
        .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
    let _mcp_registration = context
        .provide(Arc::new(GalleryMcpRegistration {
            _registration: registration,
        }))
        .map_err(|error| GalleryPluginError::Publish(error.to_string()))?;
    Ok(())
}

struct GalleryMcpRegistration {
    _registration: Arc<photag_contracts::mcp::McpToolRegistration>,
}

impl cordis::Service for GalleryMcpRegistration {
    const NAME: &'static str = "photag/gallery-mcp-registration";
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
