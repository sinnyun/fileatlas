//! Read-only sidebar snapshot published for same-process MCP debugging.
use super::*;
use serde_json::json;

impl SidebarView {
    fn sidebar_state_signature(&self, cx: &Context<Self>) -> String {
        let drives_state = format!(
            "{:?}:{:?}:{}:{}",
            self.drives_read.value(),
            self.system_folders_read.value(),
            self.drives_read.is_pending(),
            self.system_folders_read.is_pending()
        );
        let mut collapsed = self.collapsed_sections.iter().copied().collect::<Vec<_>>();
        collapsed.sort_unstable();
        let mut expanded_paths = self
            .expanded_directories
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        expanded_paths.sort_unstable();
        let mut groups = self.collapsed_groups.iter().cloned().collect::<Vec<_>>();
        groups.sort_unstable();
        let editor = self
            .sidebar_editor
            .as_ref()
            .map(|editor| {
                format!(
                    "{editor:?}={}:{:?}",
                    self.sidebar_editor_input.read(cx).value(),
                    self.sidebar_editor_error
                )
            })
            .unwrap_or_default();
        format!(
            "{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
            self.active_page.key(),
            self.service.browser_enabled(),
            collapsed.join(","),
            self.sidebar_io_revision,
            self.folders.revision(),
            drives_state,
            format!("{expanded_paths:?}:{groups:?}"),
            editor,
            self.session
                .snapshot()
                .ok()
                .and_then(|saved| saved.selected_path)
                .unwrap_or_default(),
        ) + &format!(
            "{:?}:{:?}:{}",
            self.sidebar_save_error,
            self.snapshot.as_ref().ok().map(|s| s
                .folders
                .iter()
                .map(|n| (&n.folder.id, &n.folder.path, &n.folder.name))
                .collect::<Vec<_>>()),
            self.directory_revision
        ) + &format!(
            "{:?}:{:?}",
            self.favorite_action_error,
            self.favorite_header_bounds.read().ok().as_deref()
        ) + &self.address_snapshot(cx).to_string()
    }

    /// Publish the rendered sidebar sections as data so MCP checks do not have
    /// to infer them from screenshots. Cheap and change-gated.
    pub(super) fn publish_sidebar_state(&mut self, cx: &Context<Self>) {
        let signature = self.sidebar_state_signature(cx);
        if signature == self.sidebar_state_signature_cache {
            return;
        }
        self.sidebar_state_signature_cache = signature;
        let config = self.sidebar_config();
        let selected_path = self
            .session
            .snapshot()
            .ok()
            .and_then(|saved| saved.selected_path)
            .clone();
        let drives = match self.drives_read.value() {
            None => Vec::new(),
            Some(Err(_)) | Some(Ok(_)) => self
                .drives_read
                .value()
                .and_then(|result| result.as_ref().ok())
                .map(|drives| {
                    drives
                        .iter()
                        .map(|drive| {
                            json!({
                                "root": drive.root,
                                "letter": drive.letter.to_string(),
                                "label": drive.label,
                                "kind": format!("{:?}", drive.kind),
                                "total_bytes": drive.total_bytes,
                                "used_bytes": drive.used_bytes,
                                "usage_error":drive.usage_error,
                                "child_count":self.expanded_directories.get(&drive.root).map(Vec::len),
                                "expanded": self.expanded_directories.contains_key(&drive.root),
                                "selected": selected_path.as_deref().is_some_and(|selected|crate::sidebar_folders::in_sidebar_root(selected,&drive.root)),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        };
        let row = |path: &str, extra: serde_json::Value| {
            let mut row = match extra {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            row.insert("path".into(), serde_json::Value::String(path.to_owned()));
            row.insert(
                "expanded".into(),
                serde_json::Value::Bool(self.expanded_directories.contains_key(path)),
            );
            row.insert(
                "selected".into(),
                serde_json::Value::Bool(selected_path.as_deref().is_some_and(|selected| {
                    crate::sidebar_folders::same_sidebar_path(selected, path)
                })),
            );
            row.insert(
                "child_count".into(),
                json!(self.expanded_directories.get(path).map(Vec::len)),
            );
            serde_json::Value::Object(row)
        };
        let quick_access = config
            .quick_access
            .iter()
            .map(|entry| row(&entry.path, json!({"display_name": entry.display_name})))
            .collect::<Vec<_>>();
        let favorite_groups = config
            .favorite_groups
            .iter()
            .map(|group| {
                json!({
                    "id": group.id,
                    "name": group.name,
                    "header_bounds": self.favorite_header_bounds.read().ok().and_then(|headers| headers.get(&group.id).copied()),
                    "expanded": !self.collapsed_groups.contains(&group.id),
                    "folders": group.folders.iter().map(|folder| row(
                        &folder.path,
                        json!({
                            "id": folder.id,
                            "display_name": folder.display_name,
                        }),
                    )).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();
        let system_quick_access=self.system_folders_read.value().and_then(|r|r.as_ref().ok()).into_iter().flatten().map(|(label,path)|{
            let path=path.to_string_lossy().into_owned();json!({"label":label,"path":path,"expanded":self.expanded_directories.contains_key(&path),"selected":selected_path.as_deref().is_some_and(|selected|crate::sidebar_folders::same_sidebar_path(selected,&path))})
        }).collect::<Vec<_>>();
        let library_folders = self
            .snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .folders
                    .iter()
                    .map(|node| json!({"id": node.folder.id, "path": node.folder.path}))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.service.publish_sidebar_state(json!({
            "address_bar":self.address_snapshot(cx),
            "page": self.active_page.key(),
            "plugin_buttons": self
                .registered_sidebar_view_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            "browse_mode": self.service.browser_enabled(),
            "drives_collapsed": self.collapsed_sections.contains("drives"),
            "quick_access_collapsed": self.collapsed_sections.contains("quick_access"),
            "favorites_collapsed": self.collapsed_sections.contains("favorites"),
            "folders_collapsed": self.collapsed_sections.contains("folders"),
            "show_network_drives": config.show_network_drives,
            "config_revision": self.folders.revision(),
            "save_error":self.sidebar_save_error,
            "directory_revision":self.directory_revision,
            "drives_pending": self.drives_read.is_pending(),"roots_revision":self.sidebar_io_revision,
            "favorite_current_path": self.current_favorite_path(),
            "favorite_action_error": self.favorite_action_error,
            "favorite_imported_paths": config.imported_library_paths,
            "drives": drives,
            "system_quick_access": system_quick_access,
            "quick_access": quick_access,
            "favorite_groups": favorite_groups,
            "library_folders": library_folders,
            "editor": self.sidebar_editor.as_ref().map(|editor| json!({
                "target": format!("{editor:?}"),
                "text": self.sidebar_editor_input.read(cx).value(),
                "error":self.sidebar_editor_error,
            })),
        }));
    }
}
