use super::*;

impl WorkCrateProjectsService {
    /// Project root; metadata/cache live in `.workcrate/<project UUID>`.
    pub fn portable_folder(&self, id: &str) -> Result<Option<PathBuf>, WorkCrateProjectsError> {
        Ok(self
            .locations
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?
            .get(id)
            .cloned())
    }

    /// Copy the whole feature-owned dataset beside the associated native file, then publish
    /// its locator. The old dataset is retained as recovery data; the new folder is authoritative.
    pub fn relocate_project_data(&self, id: &str) -> Result<PathBuf, WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        self.ensure_project(id)?;
        if let Some(folder) = self.portable_folder(id)? {
            return Ok(folder);
        }
        self.publish_relocation(self.read_project(id)?)
    }

    /// Caller holds the write gate; also used when explicitly rebinding a portable project.
    pub(super) fn publish_relocation(
        &self,
        mut document: ProjectDocument,
    ) -> Result<PathBuf, WorkCrateProjectsError> {
        let id = document.project_id.clone();
        let source = self.project_dir(&id)?;
        let native = document
            .native_document
            .as_ref()
            .ok_or(WorkCrateProjectsError::Invalid(
                "native document not associated",
            ))?;
        let folder = native
            .parent()
            .ok_or(WorkCrateProjectsError::Invalid(
                "native document has no folder",
            ))?
            .to_owned();
        let target = folder.join(".workcrate").join(&id);
        if target.exists() {
            return Err(WorkCrateProjectsError::Invalid(
                "项目文件夹已包含 .workcrate 数据，请使用“打开项目文件夹”恢复，或为此工程选择独立文件夹",
            ));
        }
        let project = self
            .projects(true)?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| WorkCrateProjectsError::ProjectNotFound(id.clone()))?;
        document.portable = true;
        document.project = Some(project);
        store_paths(&mut document, &folder)?;
        std::fs::create_dir_all(target.parent().ok_or(WorkCrateProjectsError::Invalid(
            "project data parent missing",
        ))?)?;
        let staging = tempfile::Builder::new()
            .prefix(".workcrate-migration-")
            .tempdir_in(&folder)?;
        management::copy_directory(&source, staging.path())?;
        // `canvas/` is the obsolete per-frame mirror older releases wrote beside each cached
        // preview; the copy carries one frame per page and the legacy source stays untouched.
        let _ = std::fs::remove_dir_all(staging.path().join("canvas"));
        atomic_write_json(&staging.path().join("project.json"), &document)?;
        std::fs::rename(staging.path(), &target)?;
        let mut next = self
            .index
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?
            .clone();
        next.locations.insert(id, folder.clone());
        if let Err(error) = atomic_write_json(&self.index_path(), &next) {
            // A complete recoverable folder remains; no source file is removed.
            return Err(error);
        }
        *self
            .locations
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)? = next.locations.clone();
        *self
            .index
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)? = next;
        self.changed();
        Ok(folder)
    }

    /// Register/reconnect the same identity after the user selects a moved project folder.
    /// Reading this metadata never starts VectorCraft or rewrites its document.
    pub fn open_project_folder(
        &self,
        folder: &Path,
    ) -> Result<Vec<WorkCrateProject>, WorkCrateProjectsError> {
        let folder = std::fs::canonicalize(folder)?;
        let mut documents = Vec::new();
        let entries = std::fs::read_dir(folder.join(".workcrate"))?;
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir()
                || Uuid::parse_str(&entry.file_name().to_string_lossy()).is_err()
            {
                continue;
            }
            let (mut document, _): (ProjectDocument, bool) =
                read_json_with_backup(&entry.path().join("project.json"))?;
            if document.version != FORMAT_VERSION {
                return Err(WorkCrateProjectsError::UnsupportedVersion(document.version));
            }
            if !document.portable || document.project_id != entry.file_name().to_string_lossy() {
                return Err(WorkCrateProjectsError::Invalid(
                    "portable project identity mismatch",
                ));
            }
            let project = document
                .project
                .as_ref()
                .ok_or(WorkCrateProjectsError::Invalid(
                    "portable project identity missing",
                ))?;
            if project.id != document.project_id {
                return Err(WorkCrateProjectsError::Invalid(
                    "portable project identity mismatch",
                ));
            }
            validate_name(&project.name)?;
            resolve_paths(&mut document, &folder)?;
            if !document
                .native_document
                .as_ref()
                .is_some_and(|p| p.is_file())
            {
                return Err(WorkCrateProjectsError::Invalid(
                    "项目文件夹中的原生工程不存在",
                ));
            }
            if let Some(presets) = &document.field_presets {
                presets::validate(presets)?;
            }
            documents.push(document);
        }
        if documents.is_empty() {
            return Err(WorkCrateProjectsError::Invalid(
                "项目文件夹中没有 WorkCrate 项目数据",
            ));
        }
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let mut next = self
            .index
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?
            .clone();
        let mut projects = Vec::new();
        for document in &documents {
            let project = document
                .project
                .as_ref()
                .ok_or(WorkCrateProjectsError::Invalid(
                    "portable project identity missing",
                ))?;
            ensure_unique_name(&next, &project.name, Some(&project.id))?;
            if let Some(current) = next.projects.iter_mut().find(|p| p.id == project.id) {
                *current = project.clone();
            } else {
                next.projects.push(project.clone());
            }
            next.locations.insert(project.id.clone(), folder.clone());
            projects.push(project.clone());
        }
        validate_index(&next)?;
        atomic_write_json(&self.index_path(), &next)?;
        *self
            .locations
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)? = next.locations.clone();
        *self
            .index
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)? = next;
        let mut presets = self
            .project_presets
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        for document in documents {
            if let Some(value) = document.field_presets {
                presets.insert(document.project_id, value);
            } else {
                presets.remove(&document.project_id);
            }
        }
        self.changed();
        Ok(projects)
    }

    pub(super) fn sync_portable_project_labels(
        &self,
        current: &ProjectIndex,
        index: &ProjectIndex,
    ) -> Result<(), WorkCrateProjectsError> {
        for project in &index.projects {
            if current.projects.iter().any(|old| old == project) {
                continue;
            }
            let Some(folder) = index.locations.get(&project.id) else {
                continue;
            };
            let path = folder
                .join(".workcrate")
                .join(&project.id)
                .join("project.json");
            let (mut document, _): (ProjectDocument, bool) = match read_json_with_backup(&path) {
                Ok(value) => value,
                Err(WorkCrateProjectsError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(WorkCrateProjectsError::Invalid(
                        "项目文件夹已移动，请先使用“打开项目文件夹”重新定位",
                    ));
                }
                Err(e) => return Err(e),
            };
            if document.project.as_ref() != Some(project) {
                document.project = Some(project.clone());
                atomic_write_json(&path, &document)?;
            }
        }
        Ok(())
    }
}

fn safe_relative(path: &Path) -> Result<(), WorkCrateProjectsError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(WorkCrateProjectsError::Invalid(
            "便携项目路径必须位于项目文件夹内部",
        ));
    }
    Ok(())
}
pub(super) fn resolve_paths(
    document: &mut ProjectDocument,
    folder: &Path,
) -> Result<(), WorkCrateProjectsError> {
    if let Some(path) = &mut document.native_document {
        safe_relative(path)?;
        *path = folder.join(&*path);
    }
    for reference in &mut document.references {
        if !Path::new(&reference.path).is_absolute() {
            safe_relative(Path::new(&reference.path))?;
            reference.path = folder.join(&reference.path).to_string_lossy().into_owned();
        }
    }
    for preview in document
        .project
        .iter()
        .filter_map(|p| p.main_image.as_ref())
        .chain(
            document
                .references
                .iter()
                .filter_map(|r| r.custom_preview.as_ref()),
        )
    {
        safe_relative(Path::new(&preview.path))?;
    }
    Ok(())
}
pub(super) fn store_paths(
    document: &mut ProjectDocument,
    folder: &Path,
) -> Result<(), WorkCrateProjectsError> {
    let relative = |path: &Path| -> Result<PathBuf, WorkCrateProjectsError> {
        let path = path
            .strip_prefix(folder)
            .map_err(|_| WorkCrateProjectsError::Invalid("便携项目资源位于项目文件夹之外"))?;
        safe_relative(path)?;
        Ok(path.to_owned())
    };
    if let Some(path) = &mut document.native_document {
        *path = relative(path)?;
    }
    for reference in &mut document.references {
        if let Ok(path) = Path::new(&reference.path).strip_prefix(folder) {
            safe_relative(path)?;
            reference.path = path.to_string_lossy().replace('\\', "/");
        }
    }
    Ok(())
}
