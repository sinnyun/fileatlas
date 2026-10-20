//! Results and errors shared by file operation callers.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkCopyOutcome {
    pub media_id: i64,
    pub copied: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkMoveOutcome {
    pub media_id: i64,
    pub moved: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkDeleteOutcome {
    pub media_id: i64,
    pub removed_from_database: bool,
    pub recycled: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteResult {
    pub removed_from_database: bool,
    pub recycled: bool,
    pub recycle_error: Option<String>,
}

impl DeleteResult {
    pub(super) fn not_found() -> Self {
        Self {
            removed_from_database: false,
            recycled: false,
            recycle_error: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileOperationError {
    #[error("media item {0} is not indexed")]
    MissingMedia(i64),
    #[error("media path has no parent directory")]
    InvalidParent,
    #[error(transparent)]
    Database(#[from] fileatlas_plugin_database::DatabaseAccessError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("operating system open request failed: {0}")]
    Open(String),
    #[error("file operation worker failed: {0}")]
    Worker(#[from] tokio::task::JoinError),
    #[error("media index event failed: {0}")]
    Event(String),
}
