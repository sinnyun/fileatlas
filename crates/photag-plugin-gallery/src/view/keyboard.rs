//! Gallery keyboard selection and rating actions.

use gpui_kit::*;

use super::GalleryView;
use crate::selection::{GalleryKey, Modifiers as SelectionModifiers, SelectionEffect};

impl GalleryView {
    pub(super) fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = match event.keystroke.key.as_str() {
            "left" => GalleryKey::Left,
            "right" => GalleryKey::Right,
            "up" => GalleryKey::Up,
            "down" => GalleryKey::Down,
            "home" => GalleryKey::Home,
            "end" => GalleryKey::End,
            "escape" => GalleryKey::Escape,
            "delete" | "backspace" => GalleryKey::Delete,
            "enter" => GalleryKey::Enter,
            "a" if event.keystroke.modifiers.control => GalleryKey::SelectAll,
            "0" => GalleryKey::Rating(0),
            "1" => GalleryKey::Rating(1),
            "2" => GalleryKey::Rating(2),
            "3" => GalleryKey::Rating(3),
            "4" => GalleryKey::Rating(4),
            "5" => GalleryKey::Rating(5),
            _ => return,
        };
        let skeletons = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.skeletons.as_slice())
            .unwrap_or_default();
        let effect = self.selection.handle_key(
            skeletons,
            6,
            key,
            SelectionModifiers {
                ctrl: event.keystroke.modifiers.control,
                alt: event.keystroke.modifiers.alt,
                shift: event.keystroke.modifiers.shift,
            },
        );
        match effect {
            SelectionEffect::ApplyRating { media_ids, rating } => {
                if let Err(error) = self.service.apply_rating(&media_ids, rating) {
                    self.snapshot = Err(error.to_string());
                } else {
                    self.reload();
                }
            }
            SelectionEffect::ConfirmDelete { .. } => self.delete_selected(cx),
            SelectionEffect::OpenPreview { .. } => self.open_selected(false, cx),
            SelectionEffect::None | SelectionEffect::ScrollTo { .. } => {}
        }
        cx.notify();
    }
}
