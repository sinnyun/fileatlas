//! Gallery selection and keyboard navigation ported from PhoTag's grid handler.

use photag_plugin_database::db::queries::MediaSkeleton;

/// Keyboard inputs relevant to the source gallery grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryKey {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    SelectAll,
    Escape,
    Enter,
    Delete,
    Backspace,
    Rating(u8),
}

/// Modifier keys as observed by the GPUI host.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Selection state retained between keyboard and pointer interactions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GallerySelection {
    pub selected_media_ids: Vec<i64>,
    pub anchor_id: Option<i64>,
}

/// Side effects the host must perform after applying a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionEffect {
    None,
    ApplyRating { media_ids: Vec<i64>, rating: u8 },
    ConfirmDelete { media_ids: Vec<i64> },
    OpenPreview { media_id: i64 },
    ScrollTo { index: usize },
}

impl GallerySelection {
    /// Apply a mouse selection using the source gallery's plain/Ctrl/Shift rules.
    pub fn select_click(
        &mut self,
        skeletons: &[MediaSkeleton],
        media_id: i64,
        modifiers: Modifiers,
    ) {
        let Some(target_index) = skeletons.iter().position(|media| media.id == media_id) else {
            return;
        };
        if modifiers.shift {
            let anchor = self
                .anchor_id
                .or_else(|| self.selected_media_ids.first().copied())
                .unwrap_or(media_id);
            let anchor_index = skeletons
                .iter()
                .position(|media| media.id == anchor)
                .unwrap_or(target_index);
            let low = anchor_index.min(target_index);
            let high = anchor_index.max(target_index);
            self.selected_media_ids = skeletons[low..=high].iter().map(|media| media.id).collect();
            self.anchor_id = Some(anchor);
        } else if modifiers.ctrl {
            if let Some(index) = self
                .selected_media_ids
                .iter()
                .position(|selected| *selected == media_id)
            {
                self.selected_media_ids.remove(index);
            } else {
                self.selected_media_ids.push(media_id);
            }
            self.anchor_id = Some(media_id);
        } else {
            self.selected_media_ids = vec![media_id];
            self.anchor_id = Some(media_id);
        }
    }

    /// Handle one key using PhoTag's original range-selection and action rules.
    pub fn handle_key(
        &mut self,
        skeletons: &[MediaSkeleton],
        columns: usize,
        key: GalleryKey,
        modifiers: Modifiers,
    ) -> SelectionEffect {
        if skeletons.is_empty() {
            return SelectionEffect::None;
        }
        if modifiers.ctrl && key == GalleryKey::SelectAll {
            self.selected_media_ids = skeletons.iter().map(|item| item.id).collect();
            self.anchor_id = skeletons.first().map(|item| item.id);
            return SelectionEffect::None;
        }
        if !modifiers.ctrl && !modifiers.alt {
            if let GalleryKey::Rating(rating @ 0..=5) = key {
                if !self.selected_media_ids.is_empty() {
                    return SelectionEffect::ApplyRating {
                        media_ids: self.selected_media_ids.clone(),
                        rating,
                    };
                }
            }
        }
        if matches!(key, GalleryKey::Delete | GalleryKey::Backspace)
            && !self.selected_media_ids.is_empty()
        {
            return SelectionEffect::ConfirmDelete {
                media_ids: self.selected_media_ids.clone(),
            };
        }
        if key == GalleryKey::Escape && !self.selected_media_ids.is_empty() {
            self.selected_media_ids.clear();
            self.anchor_id = None;
            return SelectionEffect::None;
        }
        if key == GalleryKey::Enter {
            if let Some(media_id) = self.selected_media_ids.first() {
                return SelectionEffect::OpenPreview {
                    media_id: *media_id,
                };
            }
            return SelectionEffect::None;
        }

        let columns = columns.max(1);
        let target = match key {
            GalleryKey::Left => MoveTarget::Delta(-1),
            GalleryKey::Right => MoveTarget::Delta(1),
            GalleryKey::Up => MoveTarget::Delta(-(columns as isize)),
            GalleryKey::Down => MoveTarget::Delta(columns as isize),
            GalleryKey::Home => MoveTarget::First,
            GalleryKey::End => MoveTarget::Last,
            _ => return SelectionEffect::None,
        };
        let current = self
            .selected_media_ids
            .last()
            .and_then(|id| skeletons.iter().position(|item| item.id == *id))
            .or_else(|| {
                self.anchor_id
                    .and_then(|id| skeletons.iter().position(|item| item.id == id))
            });
        let target_index = match target {
            MoveTarget::First => 0,
            MoveTarget::Last => skeletons.len() - 1,
            MoveTarget::Delta(delta) => {
                let base = current.map(|index| index as isize).unwrap_or(-1);
                (base + delta).clamp(0, skeletons.len() as isize - 1) as usize
            }
        };
        let target_id = skeletons[target_index].id;
        if modifiers.shift {
            let anchor = self
                .anchor_id
                .or_else(|| self.selected_media_ids.first().copied())
                .unwrap_or(target_id);
            let anchor_index = skeletons
                .iter()
                .position(|item| item.id == anchor)
                .unwrap_or(target_index);
            if self.anchor_id.is_none() {
                self.anchor_id = Some(anchor);
            }
            let low = anchor_index.min(target_index);
            let high = anchor_index.max(target_index);
            self.selected_media_ids = skeletons[low..=high].iter().map(|item| item.id).collect();
        } else {
            self.selected_media_ids = vec![target_id];
            self.anchor_id = Some(target_id);
        }
        SelectionEffect::ScrollTo {
            index: target_index,
        }
    }
}

enum MoveTarget {
    Delta(isize),
    First,
    Last,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skeletons(count: usize) -> Vec<MediaSkeleton> {
        (0..count)
            .map(|index| MediaSkeleton {
                id: index as i64 + 100,
                effective_date: 0,
                is_video: false,
                is_audio: false,
            })
            .collect()
    }

    #[test]
    fn shift_navigation_extends_from_stable_anchor_and_scrolls() {
        let items = skeletons(10);
        let mut state = GallerySelection::default();
        assert_eq!(
            state.handle_key(&items, 4, GalleryKey::Right, Modifiers::default()),
            SelectionEffect::ScrollTo { index: 0 }
        );
        assert_eq!(state.anchor_id, Some(100));
        state.handle_key(
            &items,
            4,
            GalleryKey::Down,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(state.selected_media_ids, vec![100, 101, 102, 103, 104]);
        assert_eq!(state.anchor_id, Some(100));
    }

    #[test]
    fn select_all_rating_and_delete_match_source_actions() {
        let items = skeletons(3);
        let mut state = GallerySelection::default();
        state.handle_key(
            &items,
            1,
            GalleryKey::SelectAll,
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(state.selected_media_ids, vec![100, 101, 102]);
        assert_eq!(
            state.handle_key(&items, 1, GalleryKey::Rating(4), Modifiers::default()),
            SelectionEffect::ApplyRating {
                media_ids: vec![100, 101, 102],
                rating: 4
            }
        );
        assert_eq!(
            state.handle_key(&items, 1, GalleryKey::Delete, Modifiers::default()),
            SelectionEffect::ConfirmDelete {
                media_ids: vec![100, 101, 102]
            }
        );
    }

    #[test]
    fn pointer_selection_supports_ctrl_toggle_and_shift_ranges() {
        let items = skeletons(5);
        let mut state = GallerySelection::default();
        state.select_click(&items, 100, Modifiers::default());
        state.select_click(
            &items,
            102,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(state.selected_media_ids, vec![100, 101, 102]);
        state.select_click(
            &items,
            104,
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(state.selected_media_ids, vec![100, 101, 102, 104]);
        state.select_click(
            &items,
            101,
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            },
        );
        assert_eq!(state.selected_media_ids, vec![100, 102, 104]);
    }
}
