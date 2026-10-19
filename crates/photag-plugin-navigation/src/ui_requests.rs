use std::path::Path;

/// Requests applied by the live navigation Sidebar entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationUiRequest {
    AllFiles,
    OpenCreate,
    OpenEdit(i64),
    OpenDelete(i64),
    OpenMerge(i64),
    OpenMove(i64),
    /// Lazily load the direct children of a registered folder root.
    ExpandFolder(i64),
    /// Lazily expand a real directory inside a registered folder.
    ExpandPath(String),
    /// Select a real directory path contained by a registered folder.
    SelectPath(String),
    /// Select a real path while keeping the current Navigation sidebar page.
    SelectPathPreservingPage(String),
    CloseModal,
    /// Explorer-style sidebar sections (drives, quick access, grouped favorites).
    Sidebar(SidebarUiRequest),
    AddressBar(AddressBarRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddressBarRequest {
    Back,
    Forward,
    Up,
    Edit,
    Cancel,
    Submit,
    SetText { text: String },
    Crumb { path: String },
}

/// One semantic edit for the sidebar sections. Every variant maps onto the same
/// handler the rendered control uses; it does not bypass validation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SidebarUiRequest {
    /// Collapse or expand one section header: drives/quick_access/favorites/folders.
    ToggleSection {
        key: String,
    },
    /// Show or hide network drives in the drives section.
    SetNetworkDrives {
        show: bool,
    },
    /// Explicitly re-read drive usage and known system folders.
    RefreshRoots,
    RetrySave,
    ToggleGroup {
        group_id: String,
    },
    /// Expand or collapse one drive row and lazy-load its children.
    ToggleDrive {
        root: String,
    },
    /// Click a sidebar path row: browser mode selects; legacy indexed mode adds.
    ActivatePath {
        path: String,
    },
    /// Expand or collapse a directory row's children.
    ToggleExpanded {
        path: String,
    },
    /// Legacy indexed-mode action for adding a folder to the database.
    AddToLibrary {
        path: String,
    },
    /// Quick access "+" or editor confirm without a native picker.
    AddQuickAccess {
        path: String,
    },
    /// Menu action "Remove from quick access".
    RemoveQuickAccess {
        path: String,
    },
    /// Same handler as a group plus; resolves the committed current location at action time.
    AddCurrentFavorite {
        group_id: String,
    },
    /// Menu action "Add folder" inside a favorite group.
    AddFavoriteFolder {
        group_id: String,
        path: String,
    },
    /// Menu action "Remove from favorites".
    RemoveFavoriteFolder {
        group_id: String,
        folder_id: String,
    },
    /// Menu action "Delete group".
    RemoveFavoriteGroup {
        group_id: String,
    },
    /// Open the inline editor and prefill it, mirroring a menu action.
    OpenEditor {
        editor: SidebarEditorRequest,
        text: String,
    },
    /// Press the editor's confirm button.
    ConfirmEditor,
    /// Press the editor's cancel button.
    CancelEditor,
}

/// Sidebar editor targets addressable without a pointer hit-test.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum SidebarEditorRequest {
    AddQuickAccess,
    RenameQuickAccess { path: String },
    CreateGroup,
    RenameGroup { group_id: String },
    AddFavorite { group_id: String },
    RenameFavorite { group_id: String, folder_id: String },
}

impl NavigationUiRequest {
    pub(crate) fn target_id(&self) -> Option<i64> {
        match self {
            Self::OpenEdit(id)
            | Self::OpenDelete(id)
            | Self::OpenMerge(id)
            | Self::OpenMove(id) => Some(*id),
            Self::AllFiles
            | Self::OpenCreate
            | Self::ExpandFolder(_)
            | Self::ExpandPath(_)
            | Self::SelectPath(_)
            | Self::SelectPathPreservingPage(_)
            | Self::CloseModal
            | Self::Sidebar(_)
            | Self::AddressBar(_) => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NavigationUiRequestError {
    #[error("a navigation UI request is already pending")]
    Busy,
    #[error("tag ID must be positive, received {0}")]
    InvalidTagId(i64),
    #[error("tag {0} does not exist")]
    UnknownTag(i64),
    #[error("folder ID must be positive, received {0}")]
    InvalidFolderId(i64),
    #[error("folder {0} does not exist")]
    UnknownFolder(i64),
    #[error("directory path is not inside a registered folder: {0}")]
    InvalidDirectoryPath(String),
    #[error("could not read the tag tree")]
    TagLookupFailed,
    #[error("navigation UI request queue is unavailable")]
    QueuePoisoned,
    #[error("invalid sidebar request: {0}")]
    InvalidSidebarRequest(&'static str),
}

/// Reject sidebar edits that address unknown rows before they reach the view.
pub(crate) fn validate_sidebar_request(
    folders: &crate::SidebarFoldersService,
    request: &SidebarUiRequest,
) -> Result<(), NavigationUiRequestError> {
    use NavigationUiRequestError::InvalidSidebarRequest;
    let invalid = |reason: &'static str| Err(InvalidSidebarRequest(reason));
    let existing_dir = |path: &str| {
        if Path::new(path).is_dir() {
            Ok(())
        } else {
            invalid("path is not an existing directory")
        }
    };
    let config = folders.snapshot();
    let group = |group_id: &str| {
        config
            .favorite_groups
            .iter()
            .find(|group| group.id == group_id)
    };
    match request {
        SidebarUiRequest::ToggleSection { key } => {
            if matches!(
                key.as_str(),
                "drives" | "quick_access" | "favorites" | "folders"
            ) {
                Ok(())
            } else {
                invalid("unknown section key")
            }
        }
        SidebarUiRequest::SetNetworkDrives { .. }
        | SidebarUiRequest::RefreshRoots
        | SidebarUiRequest::RetrySave => Ok(()),
        SidebarUiRequest::ToggleDrive { root } => {
            if crate::drives::drive_root_present(root) {
                Ok(())
            } else {
                invalid("drive root is not present")
            }
        }
        SidebarUiRequest::ActivatePath { path }
        | SidebarUiRequest::ToggleExpanded { path }
        | SidebarUiRequest::AddToLibrary { path }
        | SidebarUiRequest::AddQuickAccess { path } => existing_dir(path),
        SidebarUiRequest::RemoveQuickAccess { path } => config
            .quick_access
            .iter()
            .any(|entry| &entry.path == path)
            .then_some(())
            .ok_or(InvalidSidebarRequest("path is not in quick access")),
        SidebarUiRequest::AddFavoriteFolder { group_id, path } => {
            if group(group_id).is_none() {
                return invalid("unknown favorite group");
            }
            existing_dir(path)
        }
        SidebarUiRequest::RemoveFavoriteFolder {
            group_id,
            folder_id,
        } => {
            let Some(group) = group(group_id) else {
                return invalid("unknown favorite group");
            };
            if group.folders.iter().any(|folder| &folder.id == folder_id) {
                Ok(())
            } else {
                invalid("unknown favorite folder")
            }
        }
        SidebarUiRequest::AddCurrentFavorite { group_id }
        | SidebarUiRequest::RemoveFavoriteGroup { group_id }
        | SidebarUiRequest::ToggleGroup { group_id } => {
            if group(group_id).is_some() {
                Ok(())
            } else {
                invalid("unknown favorite group")
            }
        }
        SidebarUiRequest::OpenEditor { editor, .. } => match editor {
            SidebarEditorRequest::AddQuickAccess | SidebarEditorRequest::CreateGroup => Ok(()),
            SidebarEditorRequest::RenameQuickAccess { path } => config
                .quick_access
                .iter()
                .any(|entry| &entry.path == path)
                .then_some(())
                .ok_or(InvalidSidebarRequest("path is not in quick access")),
            SidebarEditorRequest::RenameGroup { group_id }
            | SidebarEditorRequest::AddFavorite { group_id } => {
                if group(group_id).is_some() {
                    Ok(())
                } else {
                    invalid("unknown favorite group")
                }
            }
            SidebarEditorRequest::RenameFavorite {
                group_id,
                folder_id,
            } => {
                let Some(group) = group(group_id) else {
                    return invalid("unknown favorite group");
                };
                if group.folders.iter().any(|folder| &folder.id == folder_id) {
                    Ok(())
                } else {
                    invalid("unknown favorite folder")
                }
            }
        },
        SidebarUiRequest::ConfirmEditor | SidebarUiRequest::CancelEditor => Ok(()),
    }
}
