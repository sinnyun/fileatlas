use super::*;

#[test]
fn explicit_rebind_keeps_metadata_adjacent_without_touching_either_native()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (service, project, original) =
        fixture(&temp.path().join("profile"), "A", &temp.path().join("one"))?;
    service.relocate_project_data(&project.id)?;
    let previous = service.document_binding(&project.id)?;
    let directory = temp.path().join("two");
    std::fs::create_dir(&directory)?;
    let replacement = directory.join("replacement.vectorcraft");
    std::fs::write(&replacement, b"replacement")?;
    service.set_document_path(&project.id, &replacement)?;
    assert!(
        service
            .project_directory(&project.id)?
            .starts_with(std::fs::canonicalize(&directory)?)
    );
    assert_ne!(service.document_binding(&project.id)?, previous);
    assert_eq!(
        std::fs::read(&original)?,
        b"native bytes are VectorCraft-owned"
    );
    assert_eq!(std::fs::read(&replacement)?, b"replacement");
    Ok(())
}

fn fixture(
    root: &Path,
    name: &str,
    folder: &Path,
) -> Result<(WorkCrateProjectsService, WorkCrateProject, PathBuf), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(folder)?;
    let native = folder.join(format!("{name}.vectorcraft"));
    std::fs::write(&native, b"native bytes are VectorCraft-owned")?;
    let service = WorkCrateProjectsService::open_at(root)?;
    let project = service.create_project(name)?;
    service.set_document_path(&project.id, &native)?;
    Ok((service, project, native))
}

#[test]
fn metadata_follows_folder_and_reopens_without_original_registry()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let original = temp.path().join("original");
    let (service, project, native) = fixture(&temp.path().join("profile"), "Project", &original)?;
    let image = original.join("images/image.psd");
    std::fs::create_dir_all(image.parent().ok_or("parent")?)?;
    std::fs::write(&image, b"source")?;
    let refs = service.add_references(&project.id, &[image.clone()])?;
    let binding = service.document_binding(&project.id)?;
    let mut presets = service.project_field_presets(&project.id)?;
    presets.fields[0].values.push("项目选项".into());
    service.save_project_field_presets(&project.id, presets.clone())?;
    std::fs::write(
        service
            .project_directory(&project.id)?
            .join("boards-cache.json"),
        b"cached pages",
    )?;
    service.relocate_project_data(&project.id)?;
    let data = service.project_directory(&project.id)?;
    assert!(data.starts_with(std::fs::canonicalize(&original)?));
    assert_eq!(
        std::fs::read(&native)?,
        b"native bytes are VectorCraft-owned"
    );
    let raw: serde_json::Value = read_json(&data.join("project.json"))?;
    assert_eq!(raw["native_document"], "Project.vectorcraft");
    assert_eq!(raw["references"][0]["path"], "images/image.psd");
    service.rename_project(&project.id, "Renamed")?;
    service.set_archived(&project.id, true)?;
    drop(service);
    let moved = temp.path().join("moved");
    std::fs::rename(&original, &moved)?;
    let old_profile = WorkCrateProjectsService::open_at(temp.path().join("profile"))?;
    assert_eq!(
        old_profile.projects(true)?.len(),
        1,
        "missing folder must not drop index entry or stop plugin"
    );
    let recovered = old_profile.open_project_folder(&moved)?;
    assert_eq!(recovered[0].id, project.id);
    assert_eq!(recovered[0].name, "Renamed");
    assert!(recovered[0].archived);
    assert_eq!(old_profile.document_binding(&project.id)?, binding);
    assert_eq!(old_profile.project_field_presets(&project.id)?, presets);
    assert_eq!(old_profile.references(&project.id)?[0].id, refs[0].id);
    assert!(Path::new(&old_profile.references(&project.id)?[0].path).is_file());
    let fresh = WorkCrateProjectsService::open_at(temp.path().join("fresh-profile"))?;
    assert_eq!(fresh.open_project_folder(&moved)?[0].id, project.id);
    assert_eq!(fresh.project_field_presets(&project.id)?, presets);
    let trash = fresh.delete_project(&project.id)?;
    assert!(trash.join("project.json").is_file());
    assert!(moved.join("Project.vectorcraft").is_file());
    assert!(
        moved
            .join(".workcrate")
            .join(&project.id)
            .join("project.json")
            .is_file()
    );
    assert_eq!(fresh.open_project_folder(&moved)?.len(), 1);
    Ok(())
}

#[test]
fn shared_folder_projects_remain_independent_and_invalid_import_is_atomic()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let folder = temp.path().join("documents");
    let (service, a, _) = fixture(&temp.path().join("profile"), "A", &folder)?;
    service.relocate_project_data(&a.id)?;
    let b = service.create_project("B")?;
    let native = folder.join("B.vectorcraft");
    std::fs::write(&native, b"native B")?;
    service.set_document_path(&b.id, &native)?;
    service.relocate_project_data(&b.id)?;
    assert_ne!(
        service.project_directory(&a.id)?,
        service.project_directory(&b.id)?
    );
    let fresh = WorkCrateProjectsService::open_at(temp.path().join("fresh"))?;
    assert_eq!(fresh.open_project_folder(&folder)?.len(), 2);
    let path = service.project_directory(&b.id)?.join("project.json");
    let mut raw: serde_json::Value = read_json(&path)?;
    raw["native_document"] = "../escape.vectorcraft".into();
    std::fs::write(path, serde_json::to_vec(&raw)?)?;
    let other = WorkCrateProjectsService::open_at(temp.path().join("other"))?;
    assert!(other.open_project_folder(&folder).is_err());
    assert!(other.projects(true)?.is_empty());
    Ok(())
}

#[test]
fn portable_cover_and_duplicate_preserve_owned_previews() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let (service, project, _) = fixture(
        &temp.path().join("profile"),
        "A",
        &temp.path().join("documents"),
    )?;
    let directory = service.project_directory(&project.id)?;
    std::fs::write(directory.join("previews/cover.png"), b"preview")?;
    service.update_index(|index| {
        index.projects[0].main_image = Some(WorkCratePreview {
            path: "previews/cover.png".into(),
            sha256: "test".into(),
            width: 1,
            height: 1,
        });
        Ok(())
    })?;
    service.relocate_project_data(&project.id)?;
    let destination = temp.path().join("copy");
    std::fs::create_dir(&destination)?;
    let copy = service.duplicate_project_document(&project.id, "B", &destination)?;
    assert_eq!(
        std::fs::read(
            service
                .project_directory(&copy.id)?
                .join("previews/cover.png")
        )?,
        b"preview"
    );
    let fresh = WorkCrateProjectsService::open_at(temp.path().join("fresh"))?;
    assert!(
        fresh.open_project_folder(&destination)?[0]
            .main_image
            .is_some()
    );
    assert_eq!(
        fresh.document_binding(&copy.id)?,
        service.document_binding(&copy.id)?
    );
    Ok(())
}
