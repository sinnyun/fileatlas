use super::*;
use db::queries;

#[test]
fn browsing_is_ephemeral_but_bookmarks_and_annotations_survive_restart() {
    let root = std::env::temp_dir().join(format!(
        "photag-browser-store-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("tags.db");
    let database = DatabaseService::open_browser_at(&path).unwrap();
    let folder = database
        .with_writer(|conn| queries::add_folder(conn, "F:\\photos"))
        .unwrap();
    database
        .with_cache_writer(|conn| {
            queries::scan_upsert_media(
                conn,
                0,
                "one.txt",
                "F:\\photos\\one.txt",
                "file",
                3,
                10,
                None,
            )?;
            queries::scan_upsert_media(
                conn,
                0,
                "two.txt",
                "F:\\photos\\two.txt",
                "file",
                4,
                10,
                None,
            )?;
            Ok(())
        })
        .unwrap();
    let disk = Connection::open(&path).unwrap();
    assert_eq!(
        disk.query_row("SELECT count(*) FROM media", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let id = database
        .with_reader(|conn| {
            conn.query_row("SELECT id FROM media WHERE filename='one.txt'", [], |r| {
                r.get::<_, i64>(0)
            })
        })
        .unwrap();
    database
        .with_writer(|conn| {
            let tag = queries::create_tag(conn, "Keep", None)?;
            queries::assign_tags(conn, &[id], &[tag.id])
        })
        .unwrap();
    database
        .with_writer(|conn| queries::remove_folder(conn, folder.id))
        .unwrap();
    drop(database);
    let reopened = DatabaseService::open_browser_at(&path).unwrap();
    assert!(
        reopened
            .with_reader(queries::list_folders)
            .unwrap()
            .is_empty()
    );
    let rows = reopened
        .with_reader(|conn| queries::get_media(conn, &queries::MediaQuery::default()))
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].full_path, "F:\\photos\\one.txt");
    assert_eq!(
        reopened
            .with_reader(|conn| queries::get_media_tags(conn, id))
            .unwrap()
            .len(),
        1
    );
    reopened
        .forget_records(&[rows[0].full_path.clone()])
        .unwrap();
    assert_eq!(
        disk.query_row("SELECT count(*) FROM media", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(reopened);
    drop(disk);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recent_cache_eviction_preserves_history_and_literal_path_identity() {
    let root = std::env::temp_dir().join(format!(
        "photag-cache-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let database = DatabaseService::open_browser_at(root.join("tags.db")).unwrap();
    database
        .with_cache_writer(|c| {
            queries::scan_upsert_media(c, 0, "keep.txt", "F:/a_%/keep.txt", "file", 1, 1, None)?;
            queries::scan_upsert_media(c, 0, "wrong.txt", "F:/abc/wrong.txt", "file", 1, 1, None)?;
            Ok(())
        })
        .unwrap();
    let query = queries::MediaQuery {
        directory_tree: Some("F:/a_%".into()),
        ..Default::default()
    };
    let rows = database
        .with_reader(|c| queries::get_media(c, &query))
        .unwrap();
    assert_eq!(rows.len(), 1);
    database.pin_history(&[rows[0].id]).unwrap();
    for i in 0..6 {
        database
            .begin_directory_cache(&format!("F:/other{i}"))
            .unwrap();
    }
    assert!(
        database
            .with_reader(|c| queries::get_media_by_id(c, rows[0].id))
            .unwrap()
            .is_some()
    );
    database.pin_history(&[]).unwrap();
    database.begin_directory_cache("F:/last").unwrap();
    assert!(
        database
            .with_reader(|c| queries::get_media_by_id(c, rows[0].id))
            .unwrap()
            .is_none()
    );
    drop(database);
    std::fs::remove_dir_all(root).unwrap();
}
