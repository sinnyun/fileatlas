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
    CloseModal,
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
            | Self::CloseModal => None,
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
}
