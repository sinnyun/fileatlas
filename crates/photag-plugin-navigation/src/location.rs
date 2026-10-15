//! Window-local browser history and lexical path presentation. No filesystem IO.
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Location {
    Home,
    Directory(String),
    Tags {
        included: Vec<i64>,
        excluded: Vec<i64>,
    },
}
impl Location {
    pub(crate) fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Directory(a), Self::Directory(b)) => {
                crate::sidebar_folders::same_sidebar_path(a, b)
            }
            _ => self == other,
        }
    }
}

pub(crate) struct NavigationHistory {
    entries: Vec<Location>,
    cursor: usize,
}
impl NavigationHistory {
    pub fn new(initial: Location) -> Self {
        Self {
            entries: vec![initial],
            cursor: 0,
        }
    }
    pub fn current(&self) -> &Location {
        &self.entries[self.cursor]
    }
    pub fn record(&mut self, location: Location) -> bool {
        if self.current().matches(&location) {
            return false;
        }
        self.entries.truncate(self.cursor + 1);
        self.entries.push(location);
        if self.entries.len() > 200 {
            self.entries.remove(0);
        }
        self.cursor = self.entries.len() - 1;
        true
    }
    pub fn target(&self, forward: bool) -> Option<&Location> {
        if forward {
            self.entries.get(self.cursor + 1)
        } else {
            self.cursor.checked_sub(1).and_then(|i| self.entries.get(i))
        }
    }
    pub fn step(&mut self, forward: bool) {
        if self.target(forward).is_some() {
            if forward {
                self.cursor += 1;
            } else {
                self.cursor -= 1;
            }
        }
    }
}

pub(crate) fn display_path(path: &str) -> String {
    if let Some(tail) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{tail}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Crumb {
    pub label: String,
    pub path: String,
}
pub(crate) fn crumbs(path: &str) -> Vec<Crumb> {
    let path = display_path(path);
    Path::new(&path)
        .ancestors()
        .filter(|p| p.has_root())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|p| Crumb {
            label: p
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string_lossy().trim_end_matches(['\\', '/']).to_owned()),
            path: p.to_string_lossy().into_owned(),
        })
        .collect()
}
pub(crate) fn parent(path: &str) -> Option<String> {
    let parts = crumbs(path);
    parts.len().checked_sub(2).map(|i| parts[i].path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_deduplicates_bounds_and_truncates_forward_branch() {
        let mut h = NavigationHistory::new(Location::Home);
        assert!(h.target(false).is_none());
        h.record(Location::Directory(r"C:\Photos".into()));
        assert!(!h.record(Location::Directory(r"\\?\c:\photos\".into())));
        h.record(Location::Directory(r"C:\Photos\A".into()));
        h.step(false);
        assert!(h.target(true).is_some());
        h.record(Location::Directory(r"C:\Elsewhere".into()));
        assert!(h.target(true).is_none());
        for i in 0..250 {
            h.record(Location::Directory(format!(r"C:\{i}")));
        }
        assert_eq!(h.entries.len(), 200);
        for _ in 0..250 {
            h.step(false);
        }
        assert!(h.target(false).is_none());
    }
    #[cfg(windows)]
    #[test]
    fn breadcrumbs_cover_drive_and_unc_roots_without_fake_favorite_parent() {
        let c = crumbs(r"\\?\F:\AI\项目\照片");
        assert_eq!(
            c.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            vec!["F:", "AI", "项目", "照片"]
        );
        assert_eq!(c[0].path, r"F:\");
        assert_eq!(parent(r"F:\"), None);
        assert_eq!(parent(r"F:\AI"), Some(r"F:\".into()));
        let unc = crumbs(r"\\?\UNC\server\share\Photos");
        assert_eq!(unc.len(), 2);
        assert_eq!(unc[0].label, r"\\server\share");
        assert_eq!(parent(&unc[0].path), None);
    }
}
