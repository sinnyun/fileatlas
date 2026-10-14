use super::*;
#[test]
fn browser_projects_immediate_files_folders_and_safe_cache_windows() {
    let temp = std::env::temp_dir().join(format!(
        "photag-browser-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let root = temp.join("root_%");
    let sub = root.join("a_sub");
    let deep = sub.join("deeper");
    let empty = root.join("empty");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::create_dir_all(&empty).unwrap();
    let database = Arc::new(DatabaseService::open_at(temp.join("db.sqlite")).unwrap());
    let root_string = root.to_string_lossy().into_owned();
    let folder = database
        .with_writer(|c| queries::add_folder(c, &root_string))
        .unwrap();
    database
        .with_writer(|c| {
            for i in 0..245 {
                let name = format!("file-{i:03}.txt");
                let path = root.join(&name);
                std::fs::write(&path, b"file").unwrap();
                queries::insert_media(
                    c,
                    folder.id,
                    &name,
                    &path.to_string_lossy(),
                    "file",
                    4,
                    1_700_000_000,
                    None,
                    None,
                    None,
                )?;
            }
            for path in [sub.join("child.txt"), deep.join("deep.txt")] {
                std::fs::write(&path, b"nested").unwrap();
                queries::insert_media(
                    c,
                    folder.id,
                    &path.file_name().unwrap().to_string_lossy(),
                    &path.to_string_lossy(),
                    "file",
                    6,
                    1_700_000_000,
                    None,
                    None,
                    None,
                )?;
            }
            // LIKE metacharacters in the directory name must not select a sibling.
            let sibling = temp.join("root_XX");
            std::fs::create_dir_all(&sibling).unwrap();
            queries::insert_media(
                c,
                folder.id,
                "wrong.txt",
                &sibling.join("wrong.txt").to_string_lossy(),
                "file",
                1,
                1_700_000_000,
                None,
                None,
                None,
            )?;
            Ok(())
        })
        .unwrap();
    let mut service = GalleryService::new(database.clone());
    service.grouping = Some(Arc::new(photag_plugin_grouping::GroupingService));
    service.indexing = Some(Arc::new(photag_plugin_indexing::IndexingService::new(
        database,
        Arc::new(photag_plugin_thumbnails::ThumbnailService::open_at(temp.join("thumbs")).unwrap()),
    )));
    let query = MediaQuery {
        folder_id: Some(folder.id),
        ..Default::default()
    };
    let snapshot = service
        .load_grouped(query.clone(), GroupingMode::None)
        .unwrap();
    assert!(snapshot.browser.active);
    assert_eq!(
        snapshot.browser.directory.as_deref(),
        Some(root_string.as_str())
    );
    assert_eq!(snapshot.media_count, 245);
    assert_eq!(
        snapshot
            .browser
            .directories
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a_sub", "empty"]
    );
    assert!(snapshot.browser.parent.is_none());
    assert!(snapshot.browser.error.is_none());
    let window = service.ensure_window(&snapshot, 220..245).unwrap().unwrap();
    assert_eq!(window.media.len(), 200);
    assert_eq!(
        window.media.iter().map(|m| m.id).collect::<Vec<_>>(),
        snapshot.skeletons[window.offset..window.offset + 200]
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(service.load(query.clone()).unwrap().media_count, 248); // unchanged public recursive query
    assert!(
        !service
            .load_grouped(query, GroupingMode::Subfolder)
            .unwrap()
            .browser
            .active
    );
    let scoped = MediaQuery {
        path_prefix: Some(sub.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let nested = service.load_grouped(scoped, GroupingMode::None).unwrap();
    assert_eq!(nested.media_count, 1);
    assert_eq!(nested.media[0].filename, "child.txt");
    assert_eq!(nested.browser.parent.as_deref(), Some(root_string.as_str()));
    assert_eq!(nested.browser.directories.len(), 1);
    let empty_snapshot = service
        .load_grouped(
            MediaQuery {
                path_prefix: Some(empty.to_string_lossy().into_owned()),
                ..Default::default()
            },
            GroupingMode::None,
        )
        .unwrap();
    assert_eq!(empty_snapshot.media_count, 0);
    assert!(empty_snapshot.browser.directories.is_empty());
    assert!(empty_snapshot.browser.error.is_none());
    let missing = service
        .load_grouped(
            MediaQuery {
                path_prefix: Some(root.join("missing").to_string_lossy().into_owned()),
                ..Default::default()
            },
            GroupingMode::None,
        )
        .unwrap();
    assert!(missing.browser.error.is_some());
    let virtual_root = service
        .load_grouped(MediaQuery::default(), GroupingMode::None)
        .unwrap();
    assert_eq!(virtual_root.media_count, 0);
    assert_eq!(virtual_root.browser.directories.len(), 1);
    let tag_view = service
        .load_grouped(
            MediaQuery {
                tag_ids: Some(vec![999]),
                ..Default::default()
            },
            GroupingMode::None,
        )
        .unwrap();
    assert!(!tag_view.browser.active);
    assert_eq!(tag_view.media_count, 0);
    let global_search = service
        .load_grouped(
            MediaQuery {
                search: Some("file-000".into()),
                search_in_filenames: true,
                ..Default::default()
            },
            GroupingMode::None,
        )
        .unwrap();
    assert!(!global_search.browser.active);
    assert_eq!(global_search.media_count, 1);
    drop(service);
    let _ = std::fs::remove_dir_all(temp);
}
