use super::*;

#[test]
fn local_browser_is_direct_by_default_and_recursion_is_explicit_and_ephemeral() {
    let root = std::env::temp_dir().join(format!(
        "fileatlas-live-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let folder = root.join("files");
    std::fs::create_dir_all(folder.join("empty")).unwrap();
    std::fs::create_dir_all(folder.join("nested")).unwrap();
    std::fs::write(folder.join("one.txt"), "hello").unwrap();
    std::fs::write(folder.join("nested").join("two.txt"), "nested").unwrap();
    let database = Arc::new(DatabaseService::open_browser_at(root.join("tags.db")).unwrap());
    let service = FileManagerService::new(database.clone());
    let query = MediaQuery {
        path_prefix: Some(
            std::fs::canonicalize(&folder)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        ),
        ..Default::default()
    };
    let load = |expected| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            service.poll_directory();
            let snapshot = service
                .load_grouped(query.clone(), GroupingMode::None)
                .unwrap();
            if !service.directory_status().2 {
                assert_eq!(snapshot.media_count, expected);
                return snapshot;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    let direct = load(1);
    assert_eq!(direct.browser.directories.len(), 2);
    service.set_show_descendants(true);
    assert_eq!(load(2).media_count, 2);
    std::fs::remove_file(folder.join("one.txt")).unwrap();
    service.invalidate_directory();
    assert_eq!(load(1).media[0].filename, "two.txt");
    let disk = rusqlite::Connection::open(root.join("tags.db")).unwrap();
    assert_eq!(
        disk.query_row("SELECT count(*) FROM media", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(service);
    drop(database);
    drop(disk);
    std::fs::remove_dir_all(root).unwrap();
}
