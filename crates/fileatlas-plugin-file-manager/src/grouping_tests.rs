use super::*;
#[test]
fn reordered_groups_keep_full_row_windows_and_filters_aligned() {
    let root = std::env::temp_dir().join(format!("fileatlas-group-window-{}", std::process::id()));
    let database = Arc::new(DatabaseService::open_at(root.join("db.sqlite")).unwrap());
    (|| -> Result<(), fileatlas_plugin_database::DatabaseAccessError> {
        let folder = database.add_folder("C:/group-test")?;
        for i in 0..245 {
            let ext = if i % 2 == 0 { "png" } else { "pdf" };
            let filename = format!("item-{i:03}.{ext}");
            database.insert_media(
                folder.id,
                &filename,
                &format!("C:/group-test/sub/{filename}"),
                "file",
                i,
                1_700_000_000,
                None,
                None,
                None,
            )?;
        }
        Ok(())
    })()
    .unwrap();
    let mut service = FileManagerService::new(database);
    service.grouping = Some(Arc::new(fileatlas_plugin_grouping::GroupingService));
    let snapshot = service
        .load_grouped(MediaQuery::default(), GroupingMode::FileType)
        .unwrap();
    assert_eq!(
        snapshot.groups,
        vec![("PDF".into(), 122), ("PNG".into(), 123)]
    );
    assert_eq!(snapshot.skeletons.len(), 245);
    assert_eq!(
        snapshot.media.iter().map(|m| m.id).collect::<Vec<_>>(),
        snapshot.skeletons[..200]
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>()
    );
    let window = service.ensure_window(&snapshot, 220..240).unwrap().unwrap();
    assert_eq!(window.media.len(), 200);
    assert_eq!(
        window.media.iter().map(|m| m.id).collect::<Vec<_>>(),
        snapshot.skeletons[window.offset..window.offset + window.media.len()]
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>()
    );
    let mut query = MediaQuery::default();
    query.search = Some("item-000".into());
    query.search_in_filenames = true;
    let filtered = service
        .load_grouped(query, GroupingMode::Subfolder)
        .unwrap();
    assert_eq!(filtered.media_count, 1);
    assert_eq!(filtered.groups[0].1, 1);
    assert_eq!(filtered.media[0].filename, "item-000.png");
    drop(service);
    let _ = std::fs::remove_dir_all(root);
}
