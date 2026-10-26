use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub struct NotesService {
    root: PathBuf,
    revision: Arc<AtomicU64>,
    lock: Mutex<()>,
    selected: Mutex<Option<String>>,
    selected_folder: Mutex<Option<String>>,
    ui_snapshot: Mutex<Option<NotesUiSnapshot>>,
    _watcher: Mutex<notify::RecommendedWatcher>,
}

impl Service for NotesService {
    const NAME: &'static str = SERVICE;
}

impl NotesService {
    pub fn open(root: PathBuf) -> Result<Arc<Self>, NotesError> {
        fs::create_dir_all(&root)?;
        let revision = Arc::new(AtomicU64::new(1));
        let watcher_revision = Arc::clone(&revision);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    watcher_revision.fetch_add(1, Ordering::AcqRel);
                }
            })
            .map_err(|e| NotesError::Plugin(e.to_string()))?;
        notify::Watcher::watch(&mut watcher, &root, notify::RecursiveMode::Recursive)
            .map_err(|e| NotesError::Plugin(e.to_string()))?;
        Ok(Arc::new(Self {
            root,
            revision,
            lock: Mutex::new(()),
            selected: Mutex::new(None),
            selected_folder: Mutex::new(None),
            ui_snapshot: Mutex::new(None),
            _watcher: Mutex::new(watcher),
        }))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    pub fn select(&self, id_or_path: &str) {
        if let Ok(mut selected) = self.selected.lock() {
            *selected = Some(id_or_path.to_owned());
        }
    }

    pub fn selected(&self) -> Option<String> {
        self.selected.lock().ok()?.clone()
    }

    pub fn select_folder(&self, folder: Option<&str>) {
        if let Ok(mut selected) = self.selected_folder.lock() {
            *selected = folder.map(str::to_owned);
        }
    }

    pub fn selected_folder(&self) -> Option<String> {
        self.selected_folder.lock().ok()?.clone()
    }

    pub fn set_ui_snapshot(&self, snapshot: Option<NotesUiSnapshot>) {
        if let Ok(mut state) = self.ui_snapshot.lock() {
            *state = snapshot;
        }
    }

    pub fn ui_snapshot(&self) -> Option<NotesUiSnapshot> {
        self.ui_snapshot.lock().ok()?.clone()
    }

    pub fn list(&self) -> Result<Vec<NoteSummary>, NotesError> {
        let mut notes = Vec::new();
        self.walk_notes(&self.root, &mut notes)?;
        notes.sort_by_key(|note| note.path.to_lowercase());
        Ok(notes)
    }

    pub fn list_folders(&self) -> Result<Vec<NoteFolder>, NotesError> {
        let mut folders = Vec::new();
        self.walk_folders(&self.root, 0, &mut folders)?;
        folders.sort_by_key(|folder| folder.path.to_lowercase());
        Ok(folders)
    }

    pub fn read(&self, id_or_path: &str) -> Result<NoteDocument, NotesError> {
        let notes = self.list()?;
        let summary = notes
            .into_iter()
            .find(|note| note.id == id_or_path || note.path == id_or_path)
            .ok_or_else(|| NotesError::NotFound(id_or_path.into()))?;
        let full_path = self.resolve_relative(&summary.path)?;
        let content = fs::read_to_string(full_path)?;
        let (metadata, body) = split_frontmatter(&content)?;
        let backlinks = self
            .list()?
            .into_iter()
            .filter(|candidate| candidate.id != summary.id)
            .filter_map(|candidate| {
                let text = fs::read_to_string(self.resolve_relative(&candidate.path).ok()?).ok()?;
                (references_title(&text, &summary.title)).then_some(candidate)
            })
            .collect();
        let mut summary = summary;
        summary.category = metadata.category;
        summary.status = metadata.status;
        summary.tags = metadata.tags;
        Ok(NoteDocument {
            summary,
            body,
            backlinks,
        })
    }

    pub fn create(
        &self,
        title: &str,
        folder: Option<&str>,
        body: Option<&str>,
    ) -> Result<NoteSummary, NotesError> {
        let title = validate_name(title)?;
        let parent = match folder {
            Some(path) if !path.trim().is_empty() => self.resolve_relative(path)?,
            _ => self.root.clone(),
        };
        if !parent.starts_with(&self.root) {
            return Err(NotesError::InvalidName);
        }
        fs::create_dir_all(&parent)?;
        let path = parent.join(format!("{title}.md"));
        if path.exists() {
            return Err(NotesError::AlreadyExists);
        }
        let id = new_id();
        let metadata = NoteMetadata {
            id,
            ..NoteMetadata::default()
        };
        let text = compose_markdown(&metadata, body.unwrap_or(""))?;
        self.atomic_write(&path, text.as_bytes())?;
        self.changed();
        self.summary(&path)
    }

    pub fn update(
        &self,
        id: &str,
        body: &str,
        category: Option<&str>,
        status: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> Result<NoteSummary, NotesError> {
        let _guard = self.lock.lock().map_err(|_| NotesError::InvalidName)?;
        let path = self.find_path(id)?;
        let content = fs::read_to_string(&path)?;
        let (mut metadata, _) = split_frontmatter(&content)?;
        if metadata.id.is_empty() {
            metadata.id = stable_id(&self.relative(&path));
        }
        if let Some(category) = category {
            metadata.category = category.trim().to_owned();
        }
        if let Some(status) = status {
            metadata.status = normalize_status(status)?;
        }
        if let Some(tags) = tags {
            metadata.tags = normalize_tags(tags);
        }
        self.atomic_write(&path, compose_markdown(&metadata, body)?.as_bytes())?;
        self.changed();
        self.summary(&path)
    }

    pub fn create_folder(
        &self,
        parent: Option<&str>,
        name: &str,
    ) -> Result<NoteFolder, NotesError> {
        let name = validate_name(name)?;
        let base = match parent.filter(|value| !value.trim().is_empty()) {
            Some(value) => self.resolve_relative(value)?,
            None => self.root.clone(),
        };
        if !base.starts_with(&self.root) {
            return Err(NotesError::InvalidName);
        }
        let path = base.join(&name);
        fs::create_dir(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                NotesError::AlreadyExists
            } else {
                NotesError::Io(e)
            }
        })?;
        self.changed();
        Ok(NoteFolder {
            name,
            path: self.relative(&path),
            depth: path
                .strip_prefix(&self.root)
                .map_or(0, |p| p.components().count()),
        })
    }

    pub fn rename(&self, id: &str, new_title: &str) -> Result<NoteSummary, NotesError> {
        let new_title = validate_name(new_title)?;
        let source = self.find_path(id)?;
        let target = source.with_file_name(format!("{new_title}.md"));
        if target.exists() {
            return Err(NotesError::AlreadyExists);
        }
        fs::rename(&source, &target)?;
        self.rewrite_link_targets(&source, &target)?;
        self.changed();
        self.summary(&target)
    }

    pub fn move_note(&self, id: &str, folder: Option<&str>) -> Result<NoteSummary, NotesError> {
        let source = self.find_path(id)?;
        let parent = match folder.filter(|v| !v.trim().is_empty()) {
            Some(v) => self.resolve_relative(v)?,
            None => self.root.clone(),
        };
        if !parent.starts_with(&self.root) || parent.starts_with(&source) {
            return Err(NotesError::InvalidName);
        }
        fs::create_dir_all(&parent)?;
        let target = parent.join(source.file_name().ok_or(NotesError::InvalidName)?);
        if target.exists() {
            return Err(NotesError::AlreadyExists);
        }
        fs::rename(&source, &target)?;
        self.changed();
        self.summary(&target)
    }

    pub fn trash(&self, id: &str) -> Result<PathBuf, NotesError> {
        let source = self.find_path(id)?;
        let trash = self.root.join(".trash");
        fs::create_dir_all(&trash)?;
        let name = source.file_name().ok_or(NotesError::InvalidName)?;
        let mut target = trash.join(name);
        if target.exists() {
            target.set_file_name(format!(
                "{}-{}.md",
                source.file_stem().unwrap_or_default().to_string_lossy(),
                new_id()
            ));
        }
        fs::rename(source, &target)?;
        self.changed();
        Ok(target)
    }

    pub fn list_trash(&self) -> Result<Vec<NoteTrashEntry>, NotesError> {
        let trash = self.root.join(".trash");
        if !trash.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(trash)? {
            let entry = entry?;
            let path = entry.path();
            let is_folder = entry.file_type()?.is_dir();
            if !is_folder
                && !path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
            {
                continue;
            }
            let note_count = if is_folder {
                let mut notes = Vec::new();
                self.walk_notes(&path, &mut notes)?;
                notes.len()
            } else {
                1
            };
            entries.push(NoteTrashEntry {
                path: self.relative(&path),
                name: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                is_folder,
                note_count,
            });
        }
        entries.sort_by_key(|entry| entry.name.to_lowercase());
        Ok(entries)
    }

    pub fn restore(
        &self,
        trashed_path: &str,
        folder: Option<&str>,
    ) -> Result<NoteSummary, NotesError> {
        let source = self.resolve_relative(trashed_path)?;
        if source.parent() != Some(self.root.join(".trash").as_path()) || !source.is_file() {
            return Err(NotesError::InvalidName);
        }
        let destination = match folder.filter(|value| !value.trim().is_empty()) {
            Some(value) => self.resolve_relative(value)?,
            None => self.root.clone(),
        };
        if !destination.starts_with(&self.root) {
            return Err(NotesError::InvalidName);
        }
        fs::create_dir_all(&destination)?;
        let target = destination.join(source.file_name().ok_or(NotesError::InvalidName)?);
        if target.exists() {
            return Err(NotesError::AlreadyExists);
        }
        fs::rename(source, &target)?;
        self.changed();
        self.summary(&target)
    }

    pub fn restore_folder(
        &self,
        trashed_path: &str,
        parent: Option<&str>,
    ) -> Result<NoteFolder, NotesError> {
        let source = self.resolve_relative(trashed_path)?;
        if source.parent() != Some(self.root.join(".trash").as_path()) || !source.is_dir() {
            return Err(NotesError::InvalidName);
        }
        let destination = match parent.filter(|value| !value.trim().is_empty()) {
            Some(value) => self.resolve_relative(value)?,
            None => self.root.clone(),
        };
        if !destination.starts_with(&self.root) {
            return Err(NotesError::InvalidName);
        }
        fs::create_dir_all(&destination)?;
        let target = destination.join(source.file_name().ok_or(NotesError::InvalidName)?);
        if target.exists() {
            return Err(NotesError::AlreadyExists);
        }
        fs::rename(source, &target)?;
        self.changed();
        Ok(NoteFolder {
            name: target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: self.relative(&target),
            depth: target
                .strip_prefix(&self.root)
                .map_or(0, |path| path.components().count().saturating_sub(1)),
        })
    }

    pub fn delete_trash(&self, trashed_path: &str) -> Result<(), NotesError> {
        let source = self.resolve_relative(trashed_path)?;
        if source.parent() != Some(self.root.join(".trash").as_path()) || !source.exists() {
            return Err(NotesError::InvalidName);
        }
        let mut notes = Vec::new();
        if source.is_dir() {
            self.walk_notes(&source, &mut notes)?;
            fs::remove_dir_all(&source)?;
        } else {
            let summary = self.summary(&source)?;
            notes.push(summary);
            fs::remove_file(&source)?;
        }
        for note in notes {
            let assets = self.root.join("assets").join(note.id);
            if assets.exists() {
                fs::remove_dir_all(assets)?;
            }
        }
        self.changed();
        Ok(())
    }

    pub fn rename_folder(&self, folder: &str, name: &str) -> Result<NoteFolder, NotesError> {
        let source = self.resolve_relative(folder)?;
        if !source.starts_with(&self.root) || !source.is_dir() || source == self.root {
            return Err(NotesError::NotFound(folder.into()));
        }
        let name = validate_name(name)?;
        let target = source.with_file_name(&name);
        if target.exists() {
            return Err(NotesError::AlreadyExists);
        }
        fs::rename(source, &target)?;
        self.changed();
        Ok(NoteFolder {
            name,
            path: self.relative(&target),
            depth: target
                .strip_prefix(&self.root)
                .map_or(0, |p| p.components().count().saturating_sub(1)),
        })
    }

    pub fn move_folder(
        &self,
        folder: &str,
        parent: Option<&str>,
    ) -> Result<NoteFolder, NotesError> {
        let source = self.resolve_relative(folder)?;
        let target_parent = match parent.filter(|value| !value.trim().is_empty()) {
            Some(value) => self.resolve_relative(value)?,
            None => self.root.clone(),
        };
        if !source.starts_with(&self.root)
            || source == self.root
            || !source.is_dir()
            || !target_parent.starts_with(&self.root)
            || target_parent.starts_with(&source)
        {
            return Err(NotesError::InvalidName);
        }
        fs::create_dir_all(&target_parent)?;
        let target = target_parent.join(source.file_name().ok_or(NotesError::InvalidName)?);
        if target.exists() {
            return Err(NotesError::AlreadyExists);
        }
        fs::rename(source, &target)?;
        self.changed();
        Ok(NoteFolder {
            name: target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: self.relative(&target),
            depth: target
                .strip_prefix(&self.root)
                .map_or(0, |p| p.components().count().saturating_sub(1)),
        })
    }

    pub fn trash_folder(&self, folder: &str) -> Result<String, NotesError> {
        let source = self.resolve_relative(folder)?;
        if !source.starts_with(&self.root) || source == self.root || !source.is_dir() {
            return Err(NotesError::InvalidName);
        }
        let trash = self.root.join(".trash");
        fs::create_dir_all(&trash)?;
        let mut target = trash.join(source.file_name().ok_or(NotesError::InvalidName)?);
        if target.exists() {
            target.set_file_name(format!(
                "{}-{}",
                target.file_name().unwrap_or_default().to_string_lossy(),
                new_id()
            ));
        }
        fs::rename(source, &target)?;
        self.changed();
        Ok(self.relative(&target))
    }

    pub fn search(&self, query: &str) -> Result<Vec<NoteSummary>, NotesError> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return self.list();
        }
        let notes = self
            .list()?
            .into_iter()
            .filter(|note| {
                note.title.to_lowercase().contains(&query)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query))
                    || fs::read_to_string(self.resolve_relative(&note.path).unwrap_or_default())
                        .is_ok_and(|text| text.to_lowercase().contains(&query))
            })
            .collect::<Vec<_>>();
        Ok(notes)
    }

    pub fn backlinks(&self, id: &str) -> Result<Vec<NoteSummary>, NotesError> {
        Ok(self.read(id)?.backlinks)
    }

    pub fn outline(&self, id: &str) -> Result<Vec<NoteHeading>, NotesError> {
        let document = self.read(id)?;
        let spans = editor::markdown_syntax::markdown_spans(&document.body);
        let heading_markers = spans
            .into_iter()
            .filter(|span| span.kind == editor::markdown_syntax::SyntaxKind::HeadingMarker)
            .map(|span| span.range.start)
            .collect::<std::collections::HashSet<_>>();
        let mut headings = Vec::new();
        let mut offset = 0;
        for (line, raw_line) in document.body.split_inclusive('\n').enumerate() {
            let content = raw_line.trim_end_matches('\n').trim_end_matches('\r');
            let level = content.bytes().take_while(|byte| *byte == b'#').count();
            if (1..=6).contains(&level)
                && content.as_bytes().get(level) == Some(&b' ')
                && heading_markers.contains(&(offset + content.len() - content.trim_start().len()))
            {
                headings.push(NoteHeading {
                    level,
                    title: content[level + 1..].trim().to_owned(),
                    line,
                });
            }
            offset += raw_line.len();
        }
        Ok(headings)
    }

    pub fn rendered_markdown(&self, id: &str) -> Result<String, NotesError> {
        let document = self.read(id)?;
        let note_path = self.resolve_relative(&document.summary.path)?;
        self.render_markdown_body(&document.summary.id, &note_path, &document.body)
    }

    pub fn rendered_markdown_body(
        &self,
        id: &str,
        note_path: &str,
        body: &str,
    ) -> Result<String, NotesError> {
        let note_path = self.resolve_relative(note_path)?;
        if !note_path.starts_with(&self.root)
            || !note_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            return Err(NotesError::InvalidName);
        }
        self.render_markdown_body(id, &note_path, body)
    }

    fn render_markdown_body(
        &self,
        id: &str,
        note_path: &Path,
        body: &str,
    ) -> Result<String, NotesError> {
        let mut rendered = body.to_owned();
        let mut cursor = 0;
        while let Some(start) = rendered[cursor..].find("![") {
            let start = cursor + start;
            let Some(open_rel) = rendered[start..].find("](") else {
                break;
            };
            let open = start + open_rel;
            let Some(end_rel) = rendered[open + 2..].find(')') else {
                break;
            };
            let end = open + 2 + end_rel;
            let source = rendered[open + 2..end].trim();
            if source.contains("://") || source.starts_with("data:") {
                cursor = end + 1;
                continue;
            }
            let relative = Path::new(source);
            if relative.is_absolute() {
                cursor = end + 1;
                continue;
            }
            let asset_path = note_path.parent().unwrap_or(&self.root).join(relative);
            if !asset_path.starts_with(&self.root) {
                cursor = end + 1;
                continue;
            }
            let asset_fallback = relative
                .file_name()
                .map(|name| self.root.join("assets").join(id).join(name));
            let bytes = match fs::read(&asset_path).or_else(|_| {
                asset_fallback
                    .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
                    .and_then(fs::read)
            }) {
                Ok(bytes) => bytes,
                Err(_) => {
                    cursor = end + 1;
                    continue;
                }
            };
            let mime = match asset_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "bmp" => "image/bmp",
                _ => "application/octet-stream",
            };
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            rendered.replace_range(open + 2..end, &format!("data:{mime};base64,{encoded}"));
            cursor = open + 2 + format!("data:{mime};base64,{encoded}").len();
        }
        Ok(rendered)
    }

    pub fn insert_image(&self, id: &str, source: &Path) -> Result<String, NotesError> {
        let note = self.read(id)?;
        let path = self.find_path(id)?;
        let file_name = source.file_name().ok_or(NotesError::InvalidName)?;
        let safe = validate_name(&file_name.to_string_lossy())?;
        let assets = self.root.join("assets").join(&note.summary.id);
        fs::create_dir_all(&assets)?;
        let mut destination = assets.join(&safe);
        if destination.exists() {
            destination = assets.join(format!("{}-{}", new_id(), safe));
        }
        fs::copy(source, &destination)?;
        let relative = pathdiff(path.parent().unwrap_or(&self.root), &destination);
        self.changed();
        Ok(format!("![{}]({})", safe, relative.replace('\\', "/")))
    }

    fn walk_notes(&self, dir: &Path, output: &mut Vec<NoteSummary>) -> Result<(), NotesError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if entry.file_name() != ".trash" && entry.file_name() != "assets" {
                    self.walk_notes(&path, output)?;
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                if let Ok(summary) = self.summary(&path) {
                    output.push(summary);
                }
            }
        }
        Ok(())
    }

    fn walk_folders(
        &self,
        dir: &Path,
        depth: usize,
        output: &mut Vec<NoteFolder>,
    ) -> Result<(), NotesError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_dir()
                && ![".trash", "assets"].contains(&entry.file_name().to_string_lossy().as_ref())
            {
                output.push(NoteFolder {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: self.relative(&entry.path()),
                    depth,
                });
                self.walk_folders(&entry.path(), depth + 1, output)?;
            }
        }
        Ok(())
    }

    fn summary(&self, path: &Path) -> Result<NoteSummary, NotesError> {
        let text = fs::read_to_string(path)?;
        let (mut metadata, _) = split_frontmatter(&text)?;
        if metadata.id.is_empty() {
            metadata.id = stable_id(&self.relative(path));
        }
        let modified_unix_ms = path
            .metadata()?
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Ok(NoteSummary {
            id: metadata.id,
            title: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: self.relative(path),
            category: metadata.category,
            status: metadata.status,
            tags: metadata.tags,
            modified_unix_ms,
        })
    }

    fn find_path(&self, id: &str) -> Result<PathBuf, NotesError> {
        self.list()?
            .into_iter()
            .find(|note| note.id == id || note.path == id)
            .map(|note| self.root.join(note.path))
            .ok_or_else(|| NotesError::NotFound(id.into()))
    }

    fn resolve_relative(&self, relative: &str) -> Result<PathBuf, NotesError> {
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(NotesError::InvalidName);
        }
        Ok(self.root.join(relative))
    }

    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn atomic_write(&self, target: &Path, bytes: &[u8]) -> Result<(), NotesError> {
        let temp = target.with_extension(format!("md.tmp-{}", new_id()));
        fs::write(&temp, bytes)?;
        if let Err(error) = fs::rename(&temp, target) {
            let _ = fs::remove_file(&temp);
            return Err(NotesError::Io(error));
        }
        Ok(())
    }

    fn rewrite_link_targets(&self, old: &Path, new: &Path) -> Result<(), NotesError> {
        let old_title = old.file_stem().unwrap_or_default().to_string_lossy();
        let new_title = new.file_stem().unwrap_or_default().to_string_lossy();
        for note in self.list()? {
            let path = self.root.join(note.path);
            let content = fs::read_to_string(&path)?;
            let updated = content.replace(&format!("[[{old_title}]]"), &format!("[[{new_title}]]"));
            if updated != content {
                self.atomic_write(&path, updated.as_bytes())?;
            }
        }
        Ok(())
    }

    fn changed(&self) {
        self.revision.fetch_add(1, Ordering::AcqRel);
    }
}

fn split_frontmatter(text: &str) -> Result<(NoteMetadata, String), NotesError> {
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let mut metadata: NoteMetadata = serde_json::from_str(&rest[..end])?;
            if metadata.status.trim().is_empty() {
                metadata.status = default_status();
            }
            return Ok((metadata, rest[end + 5..].to_owned()));
        }
    }
    Ok((
        NoteMetadata {
            id: String::new(),
            ..NoteMetadata::default()
        },
        text.to_owned(),
    ))
}

fn compose_markdown(metadata: &NoteMetadata, body: &str) -> Result<String, NotesError> {
    Ok(format!(
        "---\n{}\n---\n{}",
        serde_json::to_string_pretty(metadata)?,
        body
    ))
}

fn validate_name(value: &str) -> Result<String, NotesError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 180
        || value.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || value == "."
        || value == ".."
    {
        return Err(NotesError::InvalidName);
    }
    Ok(value.to_owned())
}

fn normalize_status(value: &str) -> Result<String, NotesError> {
    let status = value.trim().to_lowercase();
    if ["draft", "in_progress", "completed", "archived"].contains(&status.as_str()) {
        Ok(status)
    } else {
        Err(NotesError::InvalidName)
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut result = tags
        .into_iter()
        .map(|tag| tag.trim().to_owned())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    result.sort_by_key(|tag| tag.to_lowercase());
    result.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    result
}

fn references_title(markdown: &str, title: &str) -> bool {
    markdown.contains(&format!("[[{title}]]"))
}
pub(super) fn new_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("n{:x}-{:x}", now, std::process::id())
}
fn pathdiff(from: &Path, to: &Path) -> String {
    let from_parts = from
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let to_parts = to
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let shared = from_parts
        .iter()
        .zip(&to_parts)
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count();
    let mut parts = vec!["..".to_owned(); from_parts.len().saturating_sub(shared)];
    parts.extend(to_parts.into_iter().skip(shared));
    parts.join("/")
}

fn stable_id(path: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in path.to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("legacy-{hash:016x}")
}
