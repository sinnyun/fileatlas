use super::*;
#[test]
fn relinking_requires_missing_source_and_preserves_target_annotations() {
    let root = std::env::temp_dir().join(format!(
        "photag-relink-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let original = root.join("original.txt");
    let destination = root.join("new.txt");
    let rated = root.join("rated.txt");
    std::fs::write(&original, b"old").unwrap();
    std::fs::write(&destination, b"replacement").unwrap();
    std::fs::write(&rated, b"rated").unwrap();
    let original = std::fs::canonicalize(original).unwrap();
    let destination = std::fs::canonicalize(destination).unwrap();
    let rated = std::fs::canonicalize(rated).unwrap();
    let database = Arc::new(DatabaseService::open_browser_at(root.join("tags.db")).unwrap());
    let (id, rated_id) = database
        .with_writer(|conn| {
            for file in [&original, &rated] {
                queries::scan_upsert_media(
                    conn,
                    0,
                    &file.file_name().unwrap().to_string_lossy(),
                    &file.to_string_lossy(),
                    "file",
                    3,
                    0,
                    None,
                )?;
            }
            let ids = queries::get_media(conn, &queries::MediaQuery::default())?;
            let id = ids
                .iter()
                .find(|i| i.filename == "original.txt")
                .unwrap()
                .id;
            let rated_id = ids.iter().find(|i| i.filename == "rated.txt").unwrap().id;
            let tag = queries::create_tag(conn, "Keep", None)?;
            queries::assign_tags(conn, &[id], &[tag.id])?;
            queries::set_rating_bulk(conn, &[rated_id], 4)?;
            Ok((id, rated_id))
        })
        .unwrap();
    let tagging = TaggingService::new(database.clone());
    assert!(
        tagging
            .relink_record(id, &destination.to_string_lossy())
            .is_err()
    );
    std::fs::remove_file(&original).unwrap();
    assert_eq!(tagging.missing_records().unwrap().len(), 1);
    assert!(tagging.relink_record(id, &rated.to_string_lossy()).is_err());
    assert_eq!(
        database
            .with_reader(|c| queries::get_media_by_id(c, rated_id))
            .unwrap()
            .unwrap()
            .rating,
        4
    );
    tagging
        .relink_record(id, &destination.to_string_lossy())
        .unwrap();
    let record = database
        .with_reader(|c| queries::get_media_by_id(c, id))
        .unwrap()
        .unwrap();
    assert_eq!(record.full_path, destination.to_string_lossy());
    assert_eq!(record.file_size, 11);
    assert_eq!(tagging.for_media(id).unwrap().len(), 1);
    assert!(tagging.missing_records().unwrap().is_empty());
    assert_eq!(tagging.delete_missing_records(&[id]).unwrap(), 0);
    assert!(destination.exists());
    std::fs::remove_file(&destination).unwrap();
    assert_eq!(tagging.delete_missing_records(&[id]).unwrap(), 1);
    assert!(rated.exists());
    drop(tagging);
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}
