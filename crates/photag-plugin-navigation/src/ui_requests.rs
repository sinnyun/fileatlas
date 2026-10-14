/// Requests applied by the live navigation Sidebar entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationUiRequest {
    AllFiles,
    OpenCreate,
    OpenEdit(i64),
    OpenDelete(i64),
    OpenMerge(i64),
    OpenMove(i64),
    CloseModal,
}

impl NavigationUiRequest {
    pub(crate) fn target_id(&self) -> Option<i64> {
        match self {
            Self::OpenEdit(id)
            | Self::OpenDelete(id)
            | Self::OpenMerge(id)
            | Self::OpenMove(id) => Some(*id),
            Self::AllFiles | Self::OpenCreate | Self::CloseModal => None,
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
    #[error("could not read the tag tree")]
    TagLookupFailed,
    #[error("navigation UI request queue is unavailable")]
    QueuePoisoned,
}
