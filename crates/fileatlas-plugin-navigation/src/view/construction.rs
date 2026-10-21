use super::*;

impl SidebarView {
    pub(super) fn new(
        service: Arc<NavigationService>,
        cordis: CordisContext,
        tagging: Option<Arc<fileatlas_plugin_tagging::TaggingService>>,
        session: Arc<SessionService>,
        extension_views: Vec<(String, u64, AnyView)>,
        project_tree_views: Vec<(String, u64, AnyView)>,
        launcher_shortcuts: Vec<(String, u64, AnyView)>,
        restored_folder_id: Option<i64>,
        restored_path: Option<String>,
        expanded_subdirs: HashSet<String>,
        expanded_tags: HashSet<i64>,
        included_tags: Vec<i64>,
        excluded_tags: Vec<i64>,
        saved_tag_click_mode: Option<fileatlas_plugin_session::TagClickMode>,
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
        folders: Arc<crate::SidebarFoldersService>,
        sidebar_editor_input: Entity<InputState>,
        address_input: Entity<InputState>,
        address_focus: FocusHandle,
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
                        tracing::error!("stale tag expansion cleanup failed: {error}");
                    }
                }
            }
        }
        let expanded_directories = if service.browser_enabled() {
            expanded_subdirs
                .iter()
                .map(|p| (p.clone(), Vec::new()))
                .collect()
        } else {
            match &snapshot {
                Ok(snapshot) => restore_expanded_directories(&service, snapshot, &expanded_subdirs),
                Err(_) => HashMap::new(),
            }
        };
        let saved_sidebar = session.snapshot().unwrap_or_default();
        let plugin_views_revision = service.plugin_views_revision();
        let registered_sidebar_view_ids = extension_views
            .iter()
            .chain(launcher_shortcuts.iter())
            .map(|(id, _, _)| id.clone())
            .collect();
        let initial_location =
            address_bar::location_from_session(&saved_sidebar, snapshot.as_ref().ok());
        Self {
            _ui_ready_task: None,
            address: address_bar::AddressBarState::new(
                address_input,
                initial_location,
                address_focus,
            ),
            _address_subscription: None,
            folder_picker_task: None,
            locale_revision: u64::MAX,
            service: Arc::clone(&service),
            cordis,
            tagging,
            session,
            extension_views,
            project_tree_views,
            launcher_shortcuts,
            registered_sidebar_view_ids,
            plugin_views_revision,
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
            pending_directories: if service.browser_enabled() {
                expanded_subdirs.iter().cloned().collect()
            } else {
                Default::default()
            },
            directory_read: Default::default(),
            expanded_tags,
            restored_folder_id,
            restored_path,
            restore_selection_pending: true,
            preferences,
            included_tags,
            excluded_tags,
            show_tag_search_controls: false,
            new_folder_parent: None,
            exclude_mode: saved_tag_click_mode
                == Some(fileatlas_plugin_session::TagClickMode::Exclude),
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
            folders,
            sidebar_config_revision: 0,
            favorite_import_pending: true,
            favorite_action_error: None,
            favorite_header_bounds: Default::default(),
            sidebar_io_revision: 0,
            sidebar_save_error: None,
            directory_revision: 0,
            system_folders_read: Default::default(),
            _window_activation: None,
            drives_read: Default::default(),
            collapsed_sections: ["drives", "quick_access", "favorites", "folders"]
                .into_iter()
                .filter(|key| saved_sidebar.sidebar_collapsed_sections.contains(*key))
                .collect(),
            sidebar_editor: None,
            sidebar_editor_error: None,
            sidebar_editor_input,
            picker_editor: None,
            collapsed_groups: saved_sidebar.sidebar_collapsed_groups,
            sidebar_state_signature_cache: String::new(),
        }
    }
}
