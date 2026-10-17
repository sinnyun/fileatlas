use super::*;

pub(super) fn collect_tag_ids(tag: &photag_plugin_database::Tag, output: &mut HashSet<i64>) {
    output.insert(tag.id);
    for child in &tag.children {
        collect_tag_ids(child, output);
    }
}

pub(super) fn tag_matches(
    tag: &photag_plugin_database::Tag,
    query: &str,
    mode: TagSearchMatch,
) -> bool {
    if query.is_empty() {
        return true;
    }
    let name = tag.name.to_lowercase();
    let query = query.to_lowercase();
    let self_matches = match mode {
        TagSearchMatch::Contains => name.contains(&query),
        TagSearchMatch::StartsWith => name.starts_with(&query),
    };
    self_matches
        || tag
            .children
            .iter()
            .any(|child| tag_matches(child, &query, mode))
}

pub(super) fn find_tag(
    tags: &[photag_plugin_database::Tag],
    id: i64,
) -> Option<&photag_plugin_database::Tag> {
    tags.iter().find_map(|tag| {
        if tag.id == id {
            Some(tag)
        } else {
            find_tag(&tag.children, id)
        }
    })
}

pub(super) fn collect_tag_appearance_descendants(
    tag: &photag_plugin_database::Tag,
    output: &mut Vec<(i64, bool)>,
) {
    for child in &tag.children {
        output.push((child.id, child.color.is_some() || child.icon.is_some()));
        collect_tag_appearance_descendants(child, output);
    }
}

pub(super) fn restore_expanded_directories(
    service: &NavigationService,
    snapshot: &NavigationSnapshot,
    requested: &HashSet<String>,
) -> HashMap<String, Vec<SubdirectoryNode>> {
    let mut expanded = HashMap::new();
    for path in requested {
        let target = Path::new(path);
        if !target.is_dir() {
            continue;
        }
        let Some(folder) = snapshot
            .folders
            .iter()
            .find(|folder| target.starts_with(Path::new(&folder.folder.path)))
        else {
            continue;
        };
        let root = Path::new(&folder.folder.path);
        let Ok(relative) = target.strip_prefix(root) else {
            continue;
        };
        let mut current = root.to_path_buf();
        let mut reachable = true;
        for component in relative.components() {
            let children = service.subdirectories(&current.to_string_lossy());
            let next = current.join(component);
            if requested.contains(&current.to_string_lossy().into_owned()) {
                expanded.insert(current.to_string_lossy().into_owned(), children.clone());
            }
            if !children.iter().any(|child| Path::new(&child.path) == next) {
                reachable = false;
                break;
            }
            current = next;
        }
        if reachable && requested.contains(path) {
            let children = service.subdirectories(path);
            expanded.insert(path.clone(), children);
        }
    }
    expanded
}
