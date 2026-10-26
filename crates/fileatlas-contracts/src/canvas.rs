//! Framework-neutral contribution metadata for the FileAtlas canvas host.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanvasToolGroup {
    Navigation,
    Drawing,
    Shapes,
    Connectors,
    Resources,
    Extensions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasToolIcon {
    Select,
    Hand,
    Pencil,
    Shapes,
    Connector,
    Text,
    Image,
    File,
}

/// An owner-registered command exposed in the unique FileAtlas canvas toolbar.
/// The owner registers `command_id` with UI Bridge as a normal Cordis command;
/// clicking the contribution raises that command through the standard event path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasToolContribution {
    pub id: String,
    pub command_id: String,
    pub label: String,
    /// FileAtlas's shared icon catalog key.
    pub icon: Option<CanvasToolIcon>,
    pub group: CanvasToolGroup,
    pub order: i32,
    pub shortcut: Option<String>,
}

impl CanvasToolContribution {
    pub fn new(
        id: impl Into<String>,
        command_id: impl Into<String>,
        label: impl Into<String>,
        group: CanvasToolGroup,
        order: i32,
    ) -> Self {
        Self {
            id: id.into(),
            command_id: command_id.into(),
            label: label.into(),
            icon: None,
            group,
            order,
            shortcut: None,
        }
    }
}
