//! Address/history actions belong to Navigation; Shell only hosts this view.
use super::*;
use crate::AddressBarRequest as R;
use crate::location::{Location, NavigationHistory, crumbs, display_path, parent};
use gpui_kit::component::input::InputEvent;
mod render;

pub(super) struct AddressBarState {
    pub input: Entity<InputState>,
    pub editing: bool,
    pub error: Option<String>,
    pub history: NavigationHistory,
    pub focus: FocusHandle,
}
impl AddressBarState {
    pub fn new(input: Entity<InputState>, initial: Location, focus: FocusHandle) -> Self {
        Self {
            input,
            editing: false,
            error: None,
            history: NavigationHistory::new(initial),
            focus,
        }
    }
}
pub(super) fn location_from_session(
    saved: &fileatlas_plugin_session::SessionState,
    snapshot: Option<&NavigationSnapshot>,
) -> Location {
    if let Some(path) = saved.selected_path.clone().or_else(|| {
        snapshot?
            .folders
            .iter()
            .find(|n| Some(n.folder.id) == saved.selected_folder_id)
            .map(|n| n.folder.path.clone())
    }) {
        return Location::Directory(display_path(&path));
    }
    let mut included = saved.selected_tag_ids.clone();
    let mut excluded = saved.excluded_tag_ids.clone();
    included.sort_unstable();
    included.dedup();
    excluded.sort_unstable();
    excluded.dedup();
    if included.is_empty() && excluded.is_empty() {
        Location::Home
    } else {
        Location::Tags { included, excluded }
    }
}

pub(super) fn subscribe(view: &Entity<SidebarView>, window: &mut Window, app: &mut App) {
    view.update(app, |view, cx| {
        let input = view.address.input.clone();
        view._address_subscription =
            Some(
                cx.subscribe_in(&input, window, |view, _, event: &InputEvent, window, cx| {
                    if !view.address.editing {
                        return;
                    }
                    match event {
                        InputEvent::Change => {
                            view.address.error = None;
                            cx.notify();
                        }
                        InputEvent::PressEnter { .. } => {
                            view.apply_address_request(R::Submit, window, cx)
                        }
                        _ => {}
                    }
                }),
            );
    });
}

impl SidebarView {
    pub(super) fn sync_address_location(&mut self) -> bool {
        let Ok(saved) = self.session.snapshot() else {
            return false;
        };
        let location = location_from_session(&saved, self.snapshot.as_ref().ok());
        if self.address.history.record(location) {
            self.address.editing = false;
            self.address.error = None;
            true
        } else {
            false
        }
    }
    pub(super) fn address_path(&self) -> Option<String> {
        match self.address.history.current() {
            Location::Directory(p) => Some(p.clone()),
            _ => None,
        }
    }
    fn address_failure(&mut self, message: &str, cx: &mut Context<Self>) {
        self.address.error = Some(fileatlas_ui_bridge::i18n::tr(message));
        cx.notify();
    }
    fn validate_location(&self, location: &Location) -> bool {
        match location {
            Location::Directory(path) => Path::new(path).is_dir(),
            Location::Home => true,
            // History is an action-time target check. The rendered tree can
            // predate a newly created tag or retain a since-deleted tag.
            Location::Tags { included, excluded } => self.service.snapshot().is_ok_and(|s| {
                included
                    .iter()
                    .chain(excluded)
                    .all(|id| find_tag(&s.tags, *id).is_some())
            }),
        }
    }
    fn apply_location(&mut self, location: Location, cx: &mut Context<Self>) {
        self.address.editing = false;
        self.address.error = None;
        if self.session.snapshot().ok().is_some_and(|saved| {
            location_from_session(&saved, self.snapshot.as_ref().ok()).matches(&location)
        }) {
            cx.notify();
            return;
        }
        match location {
            Location::Directory(path) => {
                self.set_navigation_page(NavigationPage::Folders);
                self.select_path(path, cx);
            }
            Location::Home => self.select_folder(None, cx),
            Location::Tags { included, excluded } => {
                self.set_navigation_page(NavigationPage::Tags);
                self.included_tags = included;
                self.excluded_tags = excluded;
                self.emit_tag_selection(cx);
            }
        }
        cx.notify();
    }
    pub(super) fn apply_address_request(
        &mut self,
        request: R,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_address_location();
        match request {
            R::Back | R::Forward => {
                let forward = matches!(request, R::Forward);
                let Some(target) = self.address.history.target(forward).cloned() else {
                    return;
                };
                if !self.validate_location(&target) {
                    self.address_failure("Navigation target is unavailable", cx);
                    return;
                }
                self.address.history.step(forward);
                self.apply_location(target, cx);
            }
            R::Up => {
                if let Some(path) = self.address_path().and_then(|p| parent(&p)) {
                    if !Path::new(&path).is_dir() {
                        self.address_failure("Folder does not exist", cx);
                        return;
                    }
                    self.apply_location(Location::Directory(path), cx);
                }
            }
            R::Crumb { path } => {
                if !self.address_path().is_some_and(|p| {
                    crumbs(&p)
                        .iter()
                        .any(|c| crate::sidebar_folders::same_sidebar_path(&c.path, &path))
                }) {
                    self.address_failure("Navigation target is unavailable", cx);
                    return;
                }
                if !Path::new(&path).is_dir() {
                    self.address_failure("Folder does not exist", cx);
                    return;
                }
                self.apply_location(Location::Directory(path), cx);
            }
            R::Edit => {
                let path = self.address_path().unwrap_or_default();
                self.address.input.update(cx, |input, cx| {
                    input.set_value(&path, window, cx);
                    input.select_all(window, cx);
                });
                self.address.editing = true;
                self.address.error = None;
                let focus = self.address.input.read(cx).focus_handle(cx);
                window.focus(&focus, cx);
                cx.notify();
            }
            R::SetText { text } => {
                if !self.address.editing {
                    self.apply_address_request(R::Edit, window, cx);
                }
                self.address
                    .input
                    .update(cx, |input, cx| input.set_value(text, window, cx));
                cx.notify();
            }
            R::Cancel => {
                self.address.editing = false;
                self.address.error = None;
                window.focus(&self.address.focus, cx);
                cx.notify();
            }
            R::Submit => {
                if !self.address.editing {
                    return;
                }
                let text = self
                    .address
                    .input
                    .read(cx)
                    .value()
                    .trim()
                    .trim_matches('"')
                    .to_owned();
                let mut path = std::path::PathBuf::from(&text);
                if text.is_empty() {
                    self.address_failure("Folder does not exist", cx);
                    return;
                }
                if !path.is_absolute() {
                    let Some(base) = self.address_path() else {
                        self.address_failure("Enter an absolute folder address", cx);
                        return;
                    };
                    path = Path::new(&base).join(path);
                }
                let Some(path) = path.canonicalize().ok().filter(|p| p.is_dir()) else {
                    self.address_failure("Folder does not exist", cx);
                    return;
                };
                self.apply_location(
                    Location::Directory(display_path(&path.to_string_lossy())),
                    cx,
                );
                window.focus(&self.address.focus, cx);
            }
        }
    }

    pub(super) fn address_snapshot(&self, cx: &Context<Self>) -> serde_json::Value {
        let path = self.address_path();
        serde_json::json!({"path":path,"editing":self.address.editing,"error":self.address.error,
            "text":if self.address.editing {Some(self.address.input.read(cx).value().to_string())}else{None},
            "can_back":self.address.history.target(false).is_some(),"can_forward":self.address.history.target(true).is_some(),
            "can_up":path.as_ref().and_then(|p|parent(p)).is_some(),
            "breadcrumbs":path.as_ref().map(|p|crumbs(p).into_iter().map(|c|serde_json::json!({"label":c.label,"path":c.path})).collect::<Vec<_>>()).unwrap_or_default(),
        })
    }
}
