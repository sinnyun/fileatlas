//! Explorer-style sidebar sections: drives, quick access, grouped favorites.
use super::*;
mod render;
use photag_contracts::{FavoriteGroup, QuickAccessEntry, SidebarFoldersConfig};
use std::time::{SystemTime, UNIX_EPOCH};

static EDITOR_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_editor_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or(0);
    let seq = EDITOR_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    format!("{prefix}-{nanos:x}-{seq:x}")
}

pub(super) fn system_quick_access() -> Vec<(&'static str, Option<std::path::PathBuf>)> {
    vec![
        ("Desktop", dirs::desktop_dir()),
        ("Documents", dirs::document_dir()),
        ("Pictures", dirs::picture_dir()),
        ("Video", dirs::video_dir()),
        ("Music", dirs::audio_dir()),
        ("Downloads", dirs::download_dir()),
    ]
}

pub(super) fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

impl SidebarView {
    pub(super) fn sidebar_config(&self) -> SidebarFoldersConfig {
        self.folders.snapshot()
    }

    pub(super) fn mutate_sidebar_config(
        &mut self,
        edit: impl FnOnce(&mut SidebarFoldersConfig),
        cx: &mut Context<Self>,
    ) {
        let updated = self.folders.update(edit);
        let had_editor = self.sidebar_editor.take().is_some();
        self.sidebar_editor_error = None;
        let Some(updated) = updated else {
            if had_editor {
                cx.notify();
            }
            return;
        };
        self.sidebar_config_revision = self.folders.revision();
        self.dispatch_sidebar_save(updated, cx);
        cx.notify();
    }

    pub(super) fn open_sidebar_editor(
        &mut self,
        editor: SidebarEditor,
        initial: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let placeholder = photag_ui_bridge::i18n::tr(editor.placeholder());
        self.sidebar_editor_input.update(cx, |input, cx| {
            input.set_value(initial, window, cx);
            input.set_placeholder(&placeholder, window, cx);
        });
        self.sidebar_editor_error = None;
        self.sidebar_editor = Some(editor);
        cx.notify();
    }

    pub(super) fn confirm_sidebar_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.sidebar_editor.clone() else {
            return;
        };
        let value = self.sidebar_editor_input.read(cx).value().trim().to_owned();
        match editor {
            SidebarEditor::AddQuickAccess => {
                if value.is_empty() || !Path::new(&value).is_dir() {
                    self.sidebar_editor_error =
                        Some(photag_ui_bridge::i18n::tr("Folder does not exist"));
                    cx.notify();
                    return;
                }
                let display_name = Path::new(&value)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
                self.mutate_sidebar_config(
                    |config| match config
                        .quick_access
                        .iter_mut()
                        .find(|entry| entry.path == value)
                    {
                        Some(_) => {}
                        None => config.quick_access.push(QuickAccessEntry {
                            path: value,
                            display_name,
                        }),
                    },
                    cx,
                );
            }
            SidebarEditor::RenameQuickAccess { path } => {
                let name = (!value.is_empty()).then_some(value);
                self.mutate_sidebar_config(
                    |config| {
                        if let Some(entry) = config
                            .quick_access
                            .iter_mut()
                            .find(|entry| entry.path == path)
                        {
                            entry.display_name = name;
                        }
                    },
                    cx,
                );
            }
            SidebarEditor::CreateGroup => {
                if value.is_empty() {
                    self.sidebar_editor_error =
                        Some(photag_ui_bridge::i18n::tr("Name cannot be empty"));
                    cx.notify();
                    return;
                }
                self.mutate_sidebar_config(
                    |config| {
                        config.favorite_groups.push(FavoriteGroup {
                            id: next_editor_id("group"),
                            name: value,
                            folders: Vec::new(),
                        })
                    },
                    cx,
                );
            }
            SidebarEditor::RenameGroup { group_id } => {
                if value.is_empty() {
                    self.sidebar_editor_error =
                        Some(photag_ui_bridge::i18n::tr("Name cannot be empty"));
                    cx.notify();
                    return;
                }
                self.mutate_sidebar_config(
                    |config| {
                        if let Some(group) =
                            config.favorite_groups.iter_mut().find(|g| g.id == group_id)
                        {
                            group.name = value;
                        }
                    },
                    cx,
                );
            }
            SidebarEditor::AddFavorite { group_id } => {
                if value.is_empty() || !Path::new(&value).is_dir() {
                    self.sidebar_editor_error =
                        Some(photag_ui_bridge::i18n::tr("Folder does not exist"));
                    cx.notify();
                    return;
                }
                let display_name = Path::new(&value)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
                self.mutate_sidebar_config(
                    |config| {
                        if let Some(group) =
                            config.favorite_groups.iter_mut().find(|g| g.id == group_id)
                            && !group.folders.iter().any(|f| f.path == value)
                        {
                            group.folders.push(photag_contracts::FavoriteFolder {
                                id: next_editor_id("fav"),
                                path: value,
                                display_name,
                            });
                        }
                    },
                    cx,
                );
            }
            SidebarEditor::RenameFavorite {
                group_id,
                folder_id,
            } => {
                let name = (!value.is_empty()).then_some(value);
                self.mutate_sidebar_config(
                    |config| {
                        if let Some(group) =
                            config.favorite_groups.iter_mut().find(|g| g.id == group_id)
                            && let Some(folder) =
                                group.folders.iter_mut().find(|f| f.id == folder_id)
                        {
                            folder.display_name = name;
                        }
                    },
                    cx,
                );
            }
        }
    }

    pub(super) fn add_quick_access_path(&mut self, path: String, cx: &mut Context<Self>) {
        let display_name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.mutate_sidebar_config(
            |config| match config
                .quick_access
                .iter_mut()
                .find(|entry| entry.path == path)
            {
                Some(_) => {}
                None => config
                    .quick_access
                    .push(QuickAccessEntry { path, display_name }),
            },
            cx,
        );
    }

    pub(super) fn add_favorite_folder_path(
        &mut self,
        group_id: String,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let display_name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.mutate_sidebar_config(
            |config| {
                if let Some(group) = config
                    .favorite_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                    && !group.folders.iter().any(|folder| folder.path == path)
                {
                    group.folders.push(photag_contracts::FavoriteFolder {
                        id: next_editor_id("fav"),
                        path,
                        display_name,
                    });
                }
            },
            cx,
        );
    }

    pub(super) fn toggle_section(&mut self, key: &'static str) {
        if !self.collapsed_sections.remove(key) {
            self.collapsed_sections.insert(key);
        }
        self.persist_sidebar_expansion();
    }

    pub(super) fn toggle_drive(&mut self, root: &str) {
        self.toggle_directory(root);
    }

    pub(super) fn refresh_sidebar_roots(&mut self) {
        self.sidebar_io_revision += 1;
        self.refresh_selected_directory();
    }

    pub(super) fn activate_sidebar_path(&mut self, path: String, cx: &mut Context<Self>) {
        if self.service.browser_enabled() {
            self.select_path(path, cx);
        } else {
            self.add_folder_path(path, cx);
        }
    }

    pub(super) fn remove_quick_access(&mut self, path: String, cx: &mut Context<Self>) {
        self.mutate_sidebar_config(
            |config| config.quick_access.retain(|entry| entry.path != path),
            cx,
        );
    }

    pub(super) fn remove_favorite_folder(
        &mut self,
        group_id: String,
        folder_id: String,
        cx: &mut Context<Self>,
    ) {
        self.mutate_sidebar_config(
            |config| {
                if let Some(group) = config
                    .favorite_groups
                    .iter_mut()
                    .find(|group| group.id == group_id)
                {
                    group.folders.retain(|folder| folder.id != folder_id);
                }
            },
            cx,
        );
    }

    pub(super) fn remove_favorite_group(&mut self, group_id: String, cx: &mut Context<Self>) {
        self.mutate_sidebar_config(
            |config| config.favorite_groups.retain(|group| group.id != group_id),
            cx,
        );
    }

    pub(super) fn set_show_network_drives(&mut self, show: bool, cx: &mut Context<Self>) {
        self.mutate_sidebar_config(|config| config.show_network_drives = show, cx);
    }

    pub(super) fn toggle_network_drives(&mut self, cx: &mut Context<Self>) {
        let show = !self.sidebar_config().show_network_drives;
        self.set_show_network_drives(show, cx);
    }

    /// Apply one semantic sidebar request through the same handlers the
    /// rendered rows and menus call.
    pub(super) fn apply_sidebar_request(
        &mut self,
        request: crate::SidebarUiRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::SidebarUiRequest as Request;
        match request {
            Request::ToggleSection { key } => {
                let static_key = match key.as_str() {
                    "drives" => "drives",
                    "quick_access" => "quick_access",
                    "favorites" => "favorites",
                    _ => "folders",
                };
                self.toggle_section(static_key);
            }
            Request::SetNetworkDrives { show } => self.set_show_network_drives(show, cx),
            Request::RefreshRoots => self.refresh_sidebar_roots(),
            Request::RetrySave => self.retry_sidebar_save(cx),
            Request::ToggleGroup { group_id } => self.toggle_favorite_group(&group_id),
            Request::ToggleDrive { root } => self.toggle_drive(&root),
            Request::ActivatePath { path } => self.activate_sidebar_path(path, cx),
            Request::ToggleExpanded { path } => self.toggle_directory(&path),
            Request::AddToLibrary { path } => self.add_folder_path(path, cx),
            Request::AddQuickAccess { path } => self.add_quick_access_path(path, cx),
            Request::RemoveQuickAccess { path } => self.remove_quick_access(path, cx),
            Request::AddFavoriteFolder { group_id, path } => {
                self.add_favorite_folder_path(group_id, path, cx);
            }
            Request::RemoveFavoriteFolder {
                group_id,
                folder_id,
            } => self.remove_favorite_folder(group_id, folder_id, cx),
            Request::RemoveFavoriteGroup { group_id } => {
                self.remove_favorite_group(group_id, cx);
            }
            Request::OpenEditor { editor, text } => {
                let target = match editor {
                    crate::SidebarEditorRequest::AddQuickAccess => SidebarEditor::AddQuickAccess,
                    crate::SidebarEditorRequest::RenameQuickAccess { path } => {
                        SidebarEditor::RenameQuickAccess { path }
                    }
                    crate::SidebarEditorRequest::CreateGroup => SidebarEditor::CreateGroup,
                    crate::SidebarEditorRequest::RenameGroup { group_id } => {
                        SidebarEditor::RenameGroup { group_id }
                    }
                    crate::SidebarEditorRequest::AddFavorite { group_id } => {
                        SidebarEditor::AddFavorite { group_id }
                    }
                    crate::SidebarEditorRequest::RenameFavorite {
                        group_id,
                        folder_id,
                    } => SidebarEditor::RenameFavorite {
                        group_id,
                        folder_id,
                    },
                };
                self.open_sidebar_editor(target, &text, window, cx);
            }
            Request::ConfirmEditor => self.confirm_sidebar_editor(cx),
            Request::CancelEditor => {
                self.sidebar_editor = None;
                self.sidebar_editor_error = None;
                cx.notify();
            }
        }
    }
}
