use super::*;
use photag_plugin_database::db::queries;
use photag_plugin_thumbnails::ThumbnailService;

fn temp_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "photag-navigation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn snapshot_and_tag_commands_preserve_tree_and_counts() {
    let root = std::env::temp_dir().join(format!(
        "photag-nav-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
    let folder_path = root.join("library").to_string_lossy().into_owned();
    let nested_path = std::path::Path::new(&folder_path).join("nested");
    std::fs::create_dir_all(&nested_path).unwrap();
    std::fs::create_dir_all(std::path::Path::new(&folder_path).join(".hidden")).unwrap();
    database
        .with_writer(|connection| {
            let folder = queries::add_folder(connection, &folder_path)?;
            queries::insert_media(
                connection,
                folder.id,
                "sample.jpg",
                &nested_path.join("sample.jpg").to_string_lossy(),
                "image",
                10,
                1_700_000_000,
                None,
                None,
                None,
            )?;
            Ok(())
        })
        .unwrap();
    let thumbnails = Arc::new(ThumbnailService::open_at(root.join("thumbs")).unwrap());
    let indexing = Arc::new(IndexingService::new(Arc::clone(&database), thumbnails));
    let service = Arc::new(NavigationService::new(database, indexing));
    let parent = service.create_tag("Events", None).unwrap();
    let child = service.create_tag("Summer", Some(parent.id)).unwrap();
    service.set_tag_display(child.id, false).unwrap();
    service
        .set_tag_appearance(child.id, Some("#1A2B3C"), Some("sun"))
        .unwrap();
    let snapshot = service.snapshot().unwrap();
    assert_eq!(snapshot.folders[0].media_count, 1);
    let children = service.subdirectories(&folder_path);
    assert_eq!(children.len(), 2);
    assert_eq!(
        children
            .iter()
            .find(|child| child.name == "nested")
            .unwrap()
            .media_count,
        1
    );
    assert_eq!(
        children
            .iter()
            .find(|child| child.name == ".hidden")
            .unwrap()
            .media_count,
        0
    );
    assert_eq!(snapshot.tags[0].children[0].name, "Summer");
    assert!(!snapshot.tags[0].children[0].display);
    service.rename_tag(child.id, "Vacation").unwrap();
    assert_eq!(
        service.snapshot().unwrap().tags[0].children[0].name,
        "Vacation"
    );
    service.delete_tag(parent.id, false).unwrap();
    assert!(service.snapshot().unwrap().tags.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn folder_add_rescan_and_remove_keep_database_and_scan_lifecycle_in_sync() {
    let root = temp_root();
    let library = root.join("library");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("sample.jpg"), b"invalid thumbnail fixture").unwrap();
    let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
    let thumbnails = Arc::new(ThumbnailService::open_at(root.join("thumbs")).unwrap());
    let indexing = Arc::new(IndexingService::new(Arc::clone(&database), thumbnails));
    let navigation = Arc::new(NavigationService::new(Arc::clone(&database), indexing));

    let (folder, progress) = navigation.add_folder(&library).unwrap();
    wait_for_scan(progress);
    assert_eq!(count_media(&database), 1);

    wait_for_scan(navigation.rescan(folder.id).unwrap());
    assert_eq!(count_media(&database), 1);
    assert_eq!(navigation.remove_folder(folder.id).unwrap().id, folder.id);
    assert_eq!(count_media(&database), 0);
    assert!(navigation.snapshot().unwrap().folders.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

fn wait_for_scan(progress: mpsc::Receiver<Option<ScanProgress>>) {
    let mut completed = false;
    while let Ok(update) = progress.recv() {
        if update.is_none() {
            completed = true;
            break;
        }
    }
    assert!(completed, "scanner closes with a completion signal");
}

fn count_media(database: &DatabaseService) -> i64 {
    database
        .with_reader(|connection| {
            connection.query_row("SELECT COUNT(*) FROM media", [], |row| row.get(0))
        })
        .unwrap()
}

#[test]
fn source_color_format_round_trips_and_rejects_invalid_input() {
    assert_eq!(parse_hex_color(" #1a2b3C "), Some([26, 43, 60]));
    assert_eq!(format_hex_color([26, 43, 60]), "#1A2B3C");
    assert_eq!(parse_hex_color("#12FG00"), None);
    assert_eq!(parse_hex_color("#123"), None);
}

#[tokio::test]
async fn navigation_plugin_registers_and_cleans_sidebar_contributions() {
    use cordis::{Context, Plugin, PreparedPlugin};
    use photag_plugin_watcher::{WatcherConfig, WatcherPlugin};
    use photag_ui_bridge::{UiBridgeService, UiCatalog};

    let root = temp_root();
    std::fs::create_dir_all(&root).unwrap();
    let context = Context::new();
    let catalog = Arc::new(UiCatalog::default());
    let _ui = context
        .provide(Arc::new(UiBridgeService::new(Arc::clone(&catalog))))
        .unwrap();
    let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
    let thumbnails = Arc::new(ThumbnailService::open_at(root.join("thumbs")).unwrap());
    let indexing = Arc::new(IndexingService::new(Arc::clone(&database), thumbnails));
    let _db = context.provide(database).unwrap();
    let _index = context.provide(indexing).unwrap();
    let _session = context
        .provide(Arc::new(photag_plugin_session::SessionService::open_at(
            root.join("session.json"),
        )))
        .unwrap();

    let watcher = WatcherPlugin;
    let watcher_input = watcher.prepare(WatcherConfig).unwrap();
    let watcher_fiber = context
        .spawn(PreparedPlugin::from_input(watcher, watcher_input))
        .await
        .unwrap();
    watcher_fiber.ready().await.unwrap();
    let navigation = NavigationPlugin;
    let navigation_input = navigation.prepare(NavigationConfig).unwrap();
    let navigation_fiber = context
        .spawn(PreparedPlugin::from_input(navigation, navigation_input))
        .await
        .unwrap();
    navigation_fiber.ready().await.unwrap();
    assert_eq!(catalog.snapshot().unwrap().len(), 3);
    navigation_fiber.dispose().await.unwrap();
    assert!(catalog.snapshot().unwrap().is_empty());
    watcher_fiber.dispose().await.unwrap();
    let _ = std::fs::remove_dir_all(root);
}
