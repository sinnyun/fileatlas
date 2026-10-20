//! Derived image cache budget; durable tag previews live in a separate directory.
use std::path::Path;
pub(super) fn prune(directory: &Path, protected: Option<&str>) {
    prune_to(directory, protected, 5000, 512 * 1024 * 1024);
}
fn prune_to(directory: &Path, protected: Option<&str>, max_files: usize, max_bytes: u64) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut files = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("jpg" | "webp")
            ) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            meta.is_file()
                .then_some((path, meta.len(), meta.modified().ok()))
        })
        .collect::<Vec<_>>();
    let mut bytes: u64 = files.iter().map(|f| f.1).sum();
    let mut count = files.len();
    files.sort_by_key(|f| f.2);
    for (path, size, _) in files {
        if count <= max_files && bytes <= max_bytes {
            break;
        }
        if protected.is_some_and(|p| Path::new(p) == path) {
            continue;
        }
        if std::fs::remove_file(path).is_ok() {
            bytes = bytes.saturating_sub(size);
            count -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_budget_retains_current_result_and_ignores_other_files() {
        let root = std::env::temp_dir().join(format!("fileatlas-budget-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for i in 0..5 {
            std::fs::write(root.join(format!("{i}.jpg")), [0u8; 5]).unwrap();
        }
        std::fs::write(root.join("keep.txt"), "keep").unwrap();
        let protected = root.join("0.jpg");
        prune_to(&root, Some(&protected.to_string_lossy()), 2, 10);
        assert!(protected.exists());
        assert!(root.join("keep.txt").exists());
        assert_eq!(
            std::fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|e| e == "jpg"))
                .count(),
            2
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
