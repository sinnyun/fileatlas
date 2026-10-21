use super::*;

impl WorkCrateProjectsService {
    pub fn set_main_image(
        &self,
        id: &str,
        path: &Path,
        thumbnails: &ThumbnailService,
    ) -> Result<(), WorkCrateProjectsError> {
        self.ensure_project(id)?;
        if canvas_image_format(path).is_none() {
            return Err(WorkCrateProjectsError::UnsupportedPreviewFormat);
        }
        let (_, width, height, _) = thumbnails
            .generate_image(path)
            .ok_or(WorkCrateProjectsError::InvalidPreview)?;
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase();
        let relative = format!("previews/cover-{}.{}", Uuid::new_v4(), extension);
        let target = self.project_dir(id)?.join(&relative);
        let sha256 = copy_hash_atomic(path, &target)?;
        self.update_index(|index| {
            let project = index
                .projects
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| WorkCrateProjectsError::ProjectNotFound(id.into()))?;
            project.main_image = Some(WorkCratePreview {
                path: relative,
                sha256,
                width,
                height,
            });
            Ok(())
        })
    }

    pub fn duplicate_project(
        &self,
        id: &str,
        name: &str,
    ) -> Result<WorkCrateProject, WorkCrateProjectsError> {
        let source = self.project_directory(id)?;
        let original = self
            .projects(true)?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| WorkCrateProjectsError::ProjectNotFound(id.into()))?;
        let created = self.create_project(name)?;
        let target = self.project_directory(&created.id)?;
        copy_directory(&source.join("previews"), &target.join("previews"))?;
        let mut document = self.read_project(id)?;
        document.project_id = created.id.clone();
        document.native_binding = Uuid::new_v4().to_string();
        if let Some(source) = document.native_document.as_ref() {
            let destination = target.join("document.vectorcraft");
            std::fs::copy(source, &destination)?;
            document.native_document = Some(destination);
        }
        self.write_project(&document)?;
        self.update_index(|index| {
            let project = index
                .projects
                .iter_mut()
                .find(|p| p.id == created.id)
                .ok_or_else(|| WorkCrateProjectsError::ProjectNotFound(created.id.clone()))?;
            project.main_image = original.main_image;
            Ok(project.clone())
        })
    }

    /// Deletion removes the index entry and retains feature-owned data in trash.
    /// Original linked files are never moved or deleted.
    pub fn delete_project(&self, id: &str) -> Result<PathBuf, WorkCrateProjectsError> {
        let source = self.project_directory(id)?;
        let trash = self.root.join("trash");
        std::fs::create_dir_all(&trash)?;
        let destination = trash.join(format!("{id}-{}", Uuid::new_v4()));
        std::fs::rename(&source, &destination)?;
        let result = self.update_index(|index| {
            index.projects.retain(|p| p.id != id);
            Ok(())
        });
        if let Err(error) = result {
            let _ = std::fs::rename(&destination, &source);
            return Err(error);
        }
        Ok(destination)
    }
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), WorkCrateProjectsError> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &target.join(entry.file_name()))?;
        } else if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), target.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_archive_and_delete_keep_original_linked_files()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source.psd");
        std::fs::write(&source, b"original")?;
        let service = WorkCrateProjectsService::open_at(temp.path().join("data"))?;
        let project = service.create_project("Original")?;
        service.add_references(&project.id, &[source.clone()])?;
        let copy = service.duplicate_project(&project.id, "Copy")?;
        assert_eq!(service.references(&copy.id)?.len(), 1);
        service.set_archived(&copy.id, true)?;
        assert_eq!(service.projects(false)?.len(), 1);
        service.set_archived(&copy.id, false)?;
        let trash = service.delete_project(&copy.id)?;
        assert!(trash.join("project.json").is_file());
        assert_eq!(std::fs::read(source)?, b"original");
        assert_eq!(service.projects(true)?.len(), 1);
        Ok(())
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    #[test]
    fn duplicate_native_document_is_owned_and_original_survives()
    -> Result<(), Box<dyn std::error::Error>> {
        let t = tempfile::tempdir()?;
        let source = t.path().join("external.vectorcraft");
        std::fs::write(&source, b"native")?;
        let service = WorkCrateProjectsService::open_at(t.path().join("data"))?;
        let p = service.create_project("A")?;
        service.set_document_path(&p.id, &source)?;
        let binding = service.document_binding(&p.id)?;
        let reopened = WorkCrateProjectsService::open_at(t.path().join("data"))?;
        assert_eq!(reopened.document_binding(&p.id)?, binding);
        service.set_document_path(&p.id, &source)?;
        assert_ne!(service.document_binding(&p.id)?, binding);
        let q = service.duplicate_project(&p.id, "B")?;
        let copied = service.document_path(&q.id)?.ok_or("native path missing")?;
        assert_ne!(copied, source);
        assert!(copied.starts_with(service.project_directory(&q.id)?));
        service.delete_project(&q.id)?;
        assert_eq!(std::fs::read(source)?, b"native");
        Ok(())
    }
}
