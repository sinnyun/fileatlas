//! FileManager keyboard selection and rating actions.

use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::FileManagerView;
use crate::selection::{
    FileManagerKey, Modifiers as SelectionModifiers, SelectionEffect, keys_match,
    should_open_preview,
};

impl FileManagerView {
    pub(super) fn handle_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let file_manager_focused = self.focus_handle.is_focused(window);
        if file_manager_focused {
            use fileatlas_plugin_navigation::{
                AddressBarRequest as A, NavigationService, NavigationUiRequest,
            };
            let request = if event.keystroke.modifiers.control
                && event.keystroke.key.eq_ignore_ascii_case("l")
            {
                Some(A::Edit)
            } else if event.keystroke.modifiers.alt {
                match event.keystroke.key.as_str() {
                    "left" => Some(A::Back),
                    "right" => Some(A::Forward),
                    _ => None,
                }
            } else {
                None
            };
            if let Some(request) = request {
                if let Ok(navigation) = self.cordis.try_service::<NavigationService>() {
                    if let Err(error) =
                        navigation.request_ui(NavigationUiRequest::AddressBar(request))
                    {
                        self.operation_status = Some(error.to_string());
                    }
                }
                cx.stop_propagation();
                return;
            }
        }
        if self.show_records && event.keystroke.key == "escape" {
            self.show_records = false;
            cx.notify();
            return;
        }
        if file_manager_focused && event.keystroke.key.eq_ignore_ascii_case("f5") {
            self.service.invalidate_directory();
            self.refresh_index();
            cx.notify();
            return;
        }
        if file_manager_focused
            && event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.shift
            && event.keystroke.key.eq_ignore_ascii_case("c")
        {
            if !self.selection.selected_media_ids.is_empty() {
                self.file_copy_buffer = self.selection.selected_media_ids.clone();
                self.operation_status = Some(format!(
                    "Ready to copy {} selected item(s). Move the pointer to a folder and press Ctrl+V.",
                    self.file_copy_buffer.len()
                ));
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if file_manager_focused
            && event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.shift
            && event.keystroke.key.eq_ignore_ascii_case("v")
        {
            if !self.file_copy_buffer.is_empty() {
                let destination = super::group_frame::destination_at(self, window.mouse_position());
                if let Some(destination) = destination {
                    self.copy_ids_to_directory(self.file_copy_buffer.clone(), destination, cx);
                } else {
                    self.operation_status = Some(
                        "Move the pointer over a folder or folder group to paste here.".into(),
                    );
                    cx.notify();
                }
            }
            cx.stop_propagation();
            return;
        }
        if file_manager_focused && event.keystroke.modifiers.alt && event.keystroke.key == "up" {
            self.directory_up(cx);
            return;
        }
        if file_manager_focused && event.keystroke.key == "enter" {
            if let Some(path) = self.selected_directory.clone() {
                self.open_directory(path, cx);
                return;
            }
        }
        if file_manager_focused
            && event.keystroke.key == "escape"
            && self.selected_directory.take().is_some()
        {
            self.inspector_deselected = true;
            cx.notify();
            return;
        }
        if event.keystroke.modifiers.control
            && !event.keystroke.modifiers.shift
            && event.keystroke.key.eq_ignore_ascii_case("z")
            && file_manager_focused
        {
            self.run_history(true, cx);
            return;
        }
        if event.keystroke.modifiers.control
            && event.keystroke.key.eq_ignore_ascii_case("y")
            && file_manager_focused
        {
            self.run_history(false, cx);
            return;
        }
        if file_manager_focused && self.bulk_action.is_none() && !self.confirm_delete {
            if self.handle_tag_hotkey(&event.keystroke.key, cx) {
                return;
            }
            if !self.selection.selected_media_ids.is_empty() {
                let configured = self.settings.as_ref().and_then(|settings| {
                    settings.snapshot().ok().map(|settings| settings.open_key)
                });
                if !event.is_held
                    && !event.keystroke.modifiers.control
                    && !event.keystroke.modifiers.alt
                    && !event.keystroke.modifiers.shift
                    && should_open_preview(configured.as_deref(), &event.keystroke.key)
                {
                    self.open_configured_target(cx);
                    return;
                }
            }
        }
        if event.keystroke.key.eq_ignore_ascii_case("escape") && self.confirm_delete {
            window.close_dialog(cx);
            self.cancel_delete_confirmation(cx);
            return;
        }
        if self.confirm_delete {
            return;
        }
        if self.bulk_action.is_some() && event.keystroke.key.eq_ignore_ascii_case("escape") {
            self.bulk_action = None;
            self.confirm_bulk_action = false;
            self.confirm_bulk_action_ids = None;
            cx.notify();
            return;
        }
        if file_manager_focused
            && self.bulk_action.is_none()
            && !self.confirm_delete
            && event.keystroke.key.eq_ignore_ascii_case("d")
            && event.keystroke.modifiers.control
            && event.keystroke.modifiers.shift
        {
            super::bulk_actions::reset_inputs(self, window, cx);
            self.bulk_action = Some(super::BulkActionKind::Delete);
            self.bulk_source = super::BulkActionSource::WholeLibrary;
            self.confirm_bulk_action = false;
            self.confirm_bulk_action_ids = None;
            cx.notify();
            return;
        }
        let key = match event.keystroke.key.as_str() {
            "left" => FileManagerKey::Left,
            "right" => FileManagerKey::Right,
            "up" => FileManagerKey::Up,
            "down" => FileManagerKey::Down,
            "home" => FileManagerKey::Home,
            "end" => FileManagerKey::End,
            "escape" => FileManagerKey::Escape,
            "delete" | "backspace" => FileManagerKey::Delete,
            "enter" => FileManagerKey::Enter,
            "a" if event.keystroke.modifiers.control => FileManagerKey::SelectAll,
            "0" => FileManagerKey::Rating(0),
            "1" => FileManagerKey::Rating(1),
            "2" => FileManagerKey::Rating(2),
            "3" => FileManagerKey::Rating(3),
            "4" => FileManagerKey::Rating(4),
            "5" => FileManagerKey::Rating(5),
            _ => return,
        };
        let skeletons = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.skeletons.as_slice())
            .unwrap_or_default();
        let effect = self.selection.handle_key(
            skeletons,
            self.columns,
            key,
            SelectionModifiers {
                ctrl: event.keystroke.modifiers.control,
                alt: event.keystroke.modifiers.alt,
                shift: event.keystroke.modifiers.shift,
            },
        );
        if matches!(key, FileManagerKey::Escape) && self.selection.selected_media_ids.is_empty() {
            self.inspector_deselected = true;
        }
        self.persist_filter_session();
        match effect {
            SelectionEffect::ApplyRating { media_ids, rating } => {
                self.apply_rating_to(&media_ids, rating);
            }
            SelectionEffect::ConfirmDelete { .. } => self.request_delete_confirmation(window, cx),
            SelectionEffect::OpenPreview { .. } => self.preview_selected(cx),
            SelectionEffect::ScrollTo { index } => {
                self.scroll_to_file_manager_item(index);
            }
            SelectionEffect::None => {}
        }
        cx.notify();
    }

    fn run_history(&mut self, undo: bool, cx: &mut Context<Self>) {
        let Some(history) = self.history.clone() else {
            return;
        };
        let has_entry = if undo {
            history.undo_description().is_some()
        } else {
            history.redo_description().is_some()
        };
        if !has_entry {
            return;
        }
        self.start_operation(
            async move {
                let message = if undo {
                    history.undo().await
                } else {
                    history.redo().await
                };
                message.unwrap_or_else(|| {
                    if undo {
                        "Nothing to undo."
                    } else {
                        "Nothing to redo."
                    }
                    .into()
                })
            },
            true,
            cx,
        );
    }

    fn handle_tag_hotkey(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(settings) = self
            .settings
            .as_ref()
            .and_then(|service| service.snapshot().ok())
        else {
            return false;
        };
        let Some(hotkey) = settings
            .tag_hotkeys
            .iter()
            .find(|hotkey| keys_match(&hotkey.key, key))
        else {
            return false;
        };
        let media_ids = self.selection.selected_media_ids.clone();
        if media_ids.is_empty() {
            return true;
        }
        let tag_id = hotkey.tag_id;
        let Some(tagging) = self.tagging.clone() else {
            self.operation_status = Some("Tagging service is unavailable.".into());
            cx.notify();
            return true;
        };
        let already_tagged = media_ids
            .first()
            .and_then(|media_id| tagging.for_media(*media_id).ok())
            .is_some_and(|tags| tags.iter().any(|tag| tag.id == tag_id));
        let result = if already_tagged {
            tagging.remove(&media_ids, &[tag_id])
        } else {
            tagging.assign(&media_ids, &[tag_id])
        };
        match result {
            Ok(()) => {
                if let Some(history) = &self.history {
                    let entry = if already_tagged {
                        fileatlas_plugin_history::UndoEntry::RemovedTags {
                            media_ids: media_ids.clone(),
                            tag_ids: vec![tag_id],
                        }
                    } else {
                        fileatlas_plugin_history::UndoEntry::AssignedTags {
                            media_ids: media_ids.clone(),
                            tag_ids: vec![tag_id],
                        }
                    };
                    history.push(entry);
                }
                self.operation_status = Some(if already_tagged {
                    format!(
                        "Removed tag from {} item{}.",
                        media_ids.len(),
                        if media_ids.len() == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "Assigned tag to {} item{}.",
                        media_ids.len(),
                        if media_ids.len() == 1 { "" } else { "s" }
                    )
                });
                self.reload();
            }
            Err(error) => self.operation_status = Some(format!("Tag update failed: {error}")),
        }
        cx.notify();
        true
    }

    fn open_configured_target(&mut self, cx: &mut Context<Self>) {
        self.preview_selected(cx);
    }
}
