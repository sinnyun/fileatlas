use super::*;

#[test]
fn project_registry_reopens_and_preserves_arbitrary_file_references() {
    let root = tempfile::tempdir().unwrap();
    let source_dir = tempfile::tempdir().unwrap();
    let source = source_dir.path().join("source.psd");
    std::fs::write(&source, b"opaque original bytes").unwrap();
    let service = WorkCrateProjectsService::open_at(root.path()).unwrap();
    let project = service.create_project("Client boards").unwrap();
    let references = service
        .add_references(&project.id, std::slice::from_ref(&source))
        .unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].display_title, "source");
    assert_eq!(
        service
            .add_references(&project.id, std::slice::from_ref(&source))
            .unwrap()
            .len(),
        1
    );
    let renamed = service.rename_project(&project.id, "Renamed").unwrap();
    assert_eq!(renamed.name, "Renamed");
    drop(service);
    let reopened = WorkCrateProjectsService::open_at(root.path()).unwrap();
    assert_eq!(reopened.projects(false).unwrap()[0].name, "Renamed");
    assert_eq!(reopened.references(&project.id).unwrap().len(), 1);
    reopened
        .remove_reference(&project.id, &references[0].id)
        .unwrap();
    assert_eq!(std::fs::read(source).unwrap(), b"opaque original bytes");
}

#[test]
fn a_missing_source_can_be_relinked_without_changing_its_asset_id() {
    let root = tempfile::tempdir().unwrap();
    let source_dir = tempfile::tempdir().unwrap();
    let old_path = source_dir.path().join("old.png");
    let new_path = source_dir.path().join("new.png");
    std::fs::write(&old_path, b"old image").unwrap();
    std::fs::write(&new_path, b"new image").unwrap();
    let service = WorkCrateProjectsService::open_at(root.path()).unwrap();
    let project = service.create_project("Relink test").unwrap();
    let asset = service
        .add_references(&project.id, &[old_path.clone()])
        .unwrap()
        .remove(0);
    std::fs::remove_file(old_path).unwrap();
    let relinked = service
        .relink_reference(&project.id, &asset.id, &new_path)
        .unwrap();
    assert_eq!(relinked.id, asset.id);
    assert_eq!(
        relinked.path,
        std::fs::canonicalize(new_path).unwrap().to_string_lossy()
    );
}

#[test]
fn custom_preview_is_decoded_copied_and_kept_after_selection_is_cleared() {
    let root = tempfile::tempdir().unwrap();
    let source_dir = tempfile::tempdir().unwrap();
    let original = source_dir.path().join("design.psd");
    let preview_path = source_dir.path().join("design-preview.png");
    std::fs::write(&original, b"preserved source bytes").unwrap();
    image::ImageBuffer::from_pixel(40, 24, image::Rgb([14u8, 90, 230]))
        .save(&preview_path)
        .unwrap();

    let service = WorkCrateProjectsService::open_at(root.path()).unwrap();
    let project = service.create_project("Preview test").unwrap();
    let asset = service
        .add_references(&project.id, std::slice::from_ref(&original))
        .unwrap()
        .remove(0);
    let thumbs = ThumbnailService::open_at(root.path().join("thumbs")).unwrap();
    let preview = service
        .set_custom_preview(&project.id, &asset.id, &preview_path, &thumbs)
        .unwrap();
    assert_eq!((preview.width, preview.height), (40, 24));
    let owned_path = root
        .path()
        .join("projects")
        .join(&project.id)
        .join(&preview.path);
    assert_eq!(
        std::fs::read(&owned_path).unwrap(),
        std::fs::read(&preview_path).unwrap()
    );
    service
        .remove_custom_preview(&project.id, &asset.id)
        .unwrap();
    assert!(owned_path.is_file());
    assert_eq!(std::fs::read(original).unwrap(), b"preserved source bytes");
}
