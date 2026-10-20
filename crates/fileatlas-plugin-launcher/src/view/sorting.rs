use super::*;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SortBy {
    Name,
    Installed,
    Size,
}
impl SortBy {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Installed => "installed",
            Self::Size => "size",
        }
    }
    pub(super) fn label(self) -> String {
        tr(match self {
            Self::Name => "Name",
            Self::Installed => "Installation date",
            Self::Size => "Size",
        })
    }
}

fn optional<T: Ord>(a: &Option<T>, b: &Option<T>, descending: bool) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => {
            if descending {
                b.cmp(a)
            } else {
                a.cmp(b)
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

impl LauncherPanel {
    pub(super) fn sorted_entries(&self, search: &str) -> Vec<LauncherEntry> {
        let mut entries = self.visible_entries(search);
        entries.sort_by(|a, b| {
            let pin_rank = |scope: crate::PinScope| match scope {
                crate::PinScope::AllPrograms => 2_u8,
                crate::PinScope::Category => 1,
                crate::PinScope::None => 0,
            };
            let pinned = pin_rank(b.pin_scope).cmp(&pin_rank(a.pin_scope));
            let name = a
                .display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase());
            let result = match self.sort_by {
                SortBy::Name => {
                    if self.descending {
                        name.reverse()
                    } else {
                        name
                    }
                }
                SortBy::Installed => optional(&a.installed_on, &b.installed_on, self.descending),
                SortBy::Size => optional(&a.size_bytes, &b.size_bytes, self.descending),
            };
            pinned.then(result).then(name).then(a.id.cmp(&b.id))
        });
        entries
    }
}
