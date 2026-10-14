use super::*;

impl SidebarView {
    pub(super) fn new(
        service: Arc<NavigationService>,
        cordis: CordisContext,
        tagging: Option<Arc<photag_plugin_tagging::TaggingService>>,
        session: Arc<SessionService>,
        extension_views: Vec<AnyView>,
        restored_folder_id: Option<i64>,
        restored_path: Option<String>,
        expanded_subdirs: HashSet<String>,
        expanded_tags: HashSet<i64>,
        included_tags: Vec<i64>,
        excluded_tags: Vec<i64>,
        saved_tag_click_mode: Option<photag_plugin_session::TagClickMode>,
        active_page: NavigationPage,
        folder_path: Entity<InputState>,
        folder_search: Entity<InputState>,
        new_folder_name: Entity<InputState>,
        tag_name: Entity<InputState>,
        tag_parent: Entity<InputState>,
        tag_edit_name: Entity<InputState>,
        tag_color: Entity<InputState>,
        tag_icon: Entity<InputState>,
        tag_move_target: Entity<InputState>,
        tag_search: Entity<InputState>,
        merge_search: Entity<InputState>,
        preferences: Arc<RwLock<TagInteractionPreferences>>,
    ) -> Self {
        let snapshot = service.snapshot().map_err(|error| error.to_string());
        let mut expanded_tags = expanded_tags;
        if let Ok(snapshot) = &snapshot {
            let mut live_tag_ids = HashSet::new();
            for tag in &snapshot.tags {
                collect_tag_ids(tag, &mut live_tag_ids);
            }
            let saved_count = expanded_tags.len();
            expanded_tags.retain(|id| live_tag_ids.contains(id));
            if expanded_tags.len() != saved_count {
                if let Ok(mut saved) = session.snapshot() {
                    saved.expanded_tag_ids = expanded_tags.clone();
                    if let Err(error) = session.replace(saved) {
                        eprintln!("stale tag expansion cleanup failed: {error}");
                    }
                }
            }
        }
        let expanded_directories = match &snapshot {
            Ok(snapshot) => restore_expanded_directories(&service, snapshot, &expanded_subdirs),
            Err(_) => HashMap::new(),
        };
        Self {
            locale_revision: u64::MAX,
            service: Arc::clone(&service),
            cordis,
            tagging,
            session,
            extension_views,
            active_page,
            folder_path,
            folder_search,
            last_folder_search_surface: String::new(),
            pending_folder_search: None,
            new_folder_name,
            tag_name,
            tag_parent,
            parent_tag_id: None,
            tag_edit_name,
            tag_color,
            tag_icon,
            tag_edit_display: false,
            tag_edit_name_only: false,
            tag_move_target,
            move_target_tag_id: None,
            tag_search,
            merge_search,
            snapshot,
            active_scan: None,
            active_scan_folder_id: None,
            scan_failure: None,
            scan_progress: None,
            expanded_directories,
            expanded_tags,
            restored_folder_id,
            restored_path,
            restore_selection_pending: true,
            preferences,
            included_tags,
            excluded_tags,
            show_folder_path_entry: false,
            show_tag_search_controls: false,
            new_folder_parent: None,
            exclude_mode: saved_tag_click_mode
                == Some(photag_plugin_session::TagClickMode::Exclude),
            tag_search_match: TagSearchMatch::Contains,
            apply_tag_appearance_to_children: false,
            exclude_edited_tag_children: true,
            associated_edit_tag: None,
            associated_tag_ids: Vec::new(),
            merge_source: None,
            move_source_id: None,
            merge_reparent_children: true,
            confirm_delete: None,
            delete_clean_parents: false,
            tag_create_open: false,
        }
    }
}
