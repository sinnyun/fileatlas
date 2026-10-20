#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum NavigationPage {
    #[default]
    Folders,
    Tags,
    Projects,
    Extensions,
}

impl NavigationPage {
    pub(super) fn from_key(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "folders" => Some(Self::Folders),
            "tags" => Some(Self::Tags),
            "projects" => Some(Self::Projects),
            "extensions" => Some(Self::Extensions),
            _ => None,
        }
    }

    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Folders => "folders",
            Self::Tags => "tags",
            Self::Projects => "projects",
            Self::Extensions => "extensions",
        }
    }
}
