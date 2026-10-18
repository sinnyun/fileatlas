use super::*;
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
        .add_folder(&folder_path)
        .and_then(|folder| {
            database.insert_media(
                folder.id,
                "sample.jpg",
                &nested_path.join("sample.jpg").to_string_lossy(),
                "image",
                10,
                1_700_000_000,
                None,
                None,
                None,
            )
        })
        .unwrap();
    let thumbnails = Arc::new(ThumbnailService::open_at(root.join("thumbs")).unwrap());
    let indexing = Arc::new(IndexingService::new(Arc::clone(&database), thumbnails));
    let service = Arc::new(NavigationService::new(
        database,
        indexing,
        Arc::new(SidebarFoldersService::default()),
    ));
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
    let navigation = Arc::new(NavigationService::new(
        Arc::clone(&database),
        indexing,
        Arc::new(SidebarFoldersService::default()),
    ));

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
    let entries = catalog.snapshot().unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.contribution.id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "navigation/sidebar-slot",
            "navigation/sidebar",
            "navigation/folders-tab",
            "navigation/launcher-shortcuts-slot"
        ])
    );
    navigation_fiber.dispose().await.unwrap();
    assert!(catalog.snapshot().unwrap().is_empty());
    watcher_fiber.dispose().await.unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn sidebar_folders_service_tracks_restore_and_update_revisions() {
    use photag_contracts::{QuickAccessEntry, SidebarFoldersConfig};

    let service = SidebarFoldersService::default();
    assert_eq!(service.revision(), 0);
    assert!(service.snapshot().show_network_drives);

    let seeded = SidebarFoldersConfig {
        quick_access: vec![QuickAccessEntry {
            path: r"X:\library".into(),
            display_name: Some("Library".into()),
        }],
        favorite_groups: Vec::new(),
        imported_library_paths: Vec::new(),
        show_network_drives: false,
    };
    service.restore(seeded.clone());
    assert_eq!(service.revision(), 1);
    service.restore(seeded.clone());
    assert_eq!(
        service.revision(),
        1,
        "identical restore does not announce a change"
    );

    let updated = service
        .update(|config| config.show_network_drives = true)
        .expect("update reports the new document");
    assert_eq!(service.revision(), 2);
    assert!(updated.show_network_drives);
    assert!(
        service
            .update(|config| config.show_network_drives = true)
            .is_none()
    );
    assert_eq!(
        service.revision(),
        2,
        "no-op edit must not publish or re-read drives"
    );
    assert_eq!(service.snapshot().quick_access.len(), 1);

    let round_trip: SidebarFoldersConfig =
        serde_json::from_str(&serde_json::to_string(&updated).unwrap()).unwrap();
    assert_eq!(round_trip, updated);
}

#[test]
fn sidebar_requests_parse_from_json_and_validate_known_targets() {
    use crate::ui_requests::validate_sidebar_request;
    use photag_contracts::{FavoriteFolder, FavoriteGroup, QuickAccessEntry, SidebarFoldersConfig};

    let here = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let folders = SidebarFoldersService::with_config(SidebarFoldersConfig {
        quick_access: vec![QuickAccessEntry {
            path: here.clone(),
            display_name: Some("here".into()),
        }],
        favorite_groups: vec![FavoriteGroup {
            id: "group-1".into(),
            name: "摄影分组".into(),
            folders: vec![FavoriteFolder {
                id: "folder-1".into(),
                path: here.clone(),
                display_name: None,
            }],
        }],
        imported_library_paths: Vec::new(),
        show_network_drives: true,
    });
    let parse = |payload: &str| -> SidebarUiRequest {
        serde_json::from_str(payload).expect("sidebar request payload")
    };
    let check = |payload: &str| validate_sidebar_request(&folders, &parse(payload));

    assert_eq!(
        parse(r#"{"kind":"remove_quick_access","path":"F:\\library"}"#),
        SidebarUiRequest::RemoveQuickAccess {
            path: r"F:\library".to_owned()
        }
    );
    assert_eq!(
        parse(r#"{"kind":"open_editor","text":"新分组","editor":{"target":"create_group"}}"#),
        SidebarUiRequest::OpenEditor {
            editor: SidebarEditorRequest::CreateGroup,
            text: "新分组".to_owned(),
        }
    );
    assert!(
        check(&format!(
            r#"{{"kind":"remove_quick_access","path":"{}"}}"#,
            here.replace('\\', "\\\\")
        ))
        .is_ok()
    );
    assert!(check(r#"{"kind":"remove_quick_access","path":"Q:\\missing"}"#).is_err());
    assert!(check(r#"{"kind":"toggle_section","key":"quick_access"}"#).is_ok());
    assert_eq!(
        check(r#"{"kind":"toggle_section","key":"nope"}"#),
        Err(NavigationUiRequestError::InvalidSidebarRequest(
            "unknown section key"
        ))
    );
    assert!(check(r#"{"kind":"set_network_drives","show":false}"#).is_ok());
    assert!(check(r#"{"kind":"add_quick_access","path":"Q:\\missing"}"#).is_err());
    assert!(
        check(r#"{"kind":"remove_favorite_folder","group_id":"group-1","folder_id":"other"}"#)
            .is_err()
    );
    assert!(check(r#"{"kind":"remove_favorite_group","group_id":"group-1"}"#).is_ok());
    assert!(check(r#"{"kind":"remove_favorite_group","group_id":"group-2"}"#).is_err());
    assert!(check(r#"{"kind":"confirm_editor"}"#).is_ok());
    let drive = (b'A'..=b'Z')
        .map(|letter| format!("{}:\\", letter as char))
        .find(|root| crate::drives::drive_root_present(root));
    if let Some(root) = drive {
        assert!(
            check(&format!(
                r#"{{"kind":"toggle_drive","root":"{}"}}"#,
                root.replace('\\', "\\\\")
            ))
            .is_ok()
        );
    }
    assert!(check(r#"{"kind":"toggle_drive","root":"not-a-drive"}"#).is_err());
}

#[test]
fn sidebar_display_paths_match_extended_windows_identity_without_io() {
    use crate::sidebar_folders::*;
    assert!(same_sidebar_path(r"\\?\F:\Photos\", r"f:/photos"));
    assert!(same_sidebar_path(
        r"\\?\UNC\server\share",
        r"\\SERVER\SHARE\"
    ));
    assert!(in_sidebar_root(r"\\?\F:\photos\one.jpg", r"F:\"));
    assert!(!in_sidebar_root(r"F:\photography", r"F:\photo"));
    assert_eq!(crate::drives::drive_root_index("F:/"), Some(5));
    assert_eq!(crate::drives::drive_root_index("F:/child"), None);
}
