use super::*;

#[test]
fn keeps_full_rows_windowed_and_loads_new_viewport_range() {
    let root = std::env::temp_dir().join(format!(
        "fileatlas-file_manager-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let database = Arc::new(DatabaseService::open_at(root.join("fileatlas.sqlite")).unwrap());
    let folder_path = root.join("library").to_string_lossy().into_owned();
    std::fs::create_dir_all(&folder_path).unwrap();
    let folder = database.add_folder(&folder_path).unwrap();
    (|| -> Result<(), fileatlas_plugin_database::DatabaseAccessError> {
        for index in 0..245 {
            let filename = format!("image-{index:03}.png");
            let path = format!("{folder_path}/{filename}");
            database.insert_media(
                folder.id,
                &filename,
                &path,
                "image",
                index,
                1_700_000_000 + index,
                None,
                None,
                None,
            )?;
        }
        Ok(())
    })()
    .unwrap();

    let file_manager = FileManagerService::new(database);
    let snapshot = file_manager.load(MediaQuery::default()).unwrap();
    assert_eq!(snapshot.media_count, 245);
    assert_eq!(snapshot.skeletons.len(), 245);
    assert_eq!(snapshot.media.len(), MEDIA_WINDOW_SIZE);
    assert_eq!(snapshot.window_offset, 0);
    assert_eq!(snapshot.media[0].filename, "image-000.png");
    assert!(
        file_manager
            .ensure_window(&snapshot, 20..30)
            .unwrap()
            .is_none()
    );

    let window = file_manager
        .ensure_window(&snapshot, 220..235)
        .unwrap()
        .unwrap();
    assert_eq!(window.media.len(), MEDIA_WINDOW_SIZE);
    assert!(window.offset <= 220);
    assert!(window.offset + window.media.len() >= 235);
    assert_eq!(window.media[220 - window.offset].filename, "image-220.png");
    let page = file_manager
        .load_page(MediaQuery::default(), MEDIA_WINDOW_SIZE)
        .unwrap();
    assert_eq!(page.offset, MEDIA_WINDOW_SIZE);
    assert_eq!(page.media.len(), 45);
    assert_eq!(page.media[0].filename, "image-200.png");
    let refreshed = file_manager
        .load_grouped_window(MediaQuery::default(), GroupingMode::Month, 200)
        .unwrap();
    assert_eq!(refreshed.window_offset, 200);
    assert_eq!(refreshed.media[0].filename, "image-200.png");
    assert_eq!(refreshed.media.len(), 45);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn applies_bulk_rating_with_source_range_clamping() {
    let root = std::env::temp_dir().join(format!(
        "fileatlas-file_manager-rating-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let database = Arc::new(DatabaseService::open_at(root.join("fileatlas.sqlite")).unwrap());
    let media_id = (|| -> Result<_, fileatlas_plugin_database::DatabaseAccessError> {
        let folder = database.add_folder("/rating-test")?;
        database.insert_media(
            folder.id,
            "sample.jpg",
            "/rating-test/sample.jpg",
            "image",
            12,
            1_700_000_000,
            None,
            None,
            None,
        )
    })()
    .unwrap();
    let file_manager = FileManagerService::new(Arc::clone(&database));
    file_manager.apply_rating(&[media_id], 6).unwrap();
    assert_eq!(
        file_manager.media_by_id(media_id).unwrap().unwrap().rating,
        5
    );
    file_manager.apply_rating(&[media_id], 0).unwrap();
    assert_eq!(
        file_manager.media_by_id(media_id).unwrap().unwrap().rating,
        0
    );
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn plugin_registers_file_manager_ui_and_cleans_it_on_disposal()
-> Result<(), Box<dyn std::error::Error>> {
    use cordis::{Context, Plugin, PreparedPlugin, Routing};
    use fileatlas_contracts::{DirectoryPathSelected, FolderSelectionChanged, TagSelectionChanged};
    use fileatlas_ui_bridge::{UiBridgeService, UiCatalog};

    let root = std::env::temp_dir().join(format!(
        "fileatlas-file_manager-plugin-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let context = Context::new();
    let catalog = Arc::new(UiCatalog::default());
    let _ui = context
        .provide(Arc::new(UiBridgeService::new(Arc::clone(&catalog))))
        .unwrap();
    let database = Arc::new(DatabaseService::open_at(root.join("fileatlas.sqlite")).unwrap());
    let _database = context.provide(Arc::clone(&database)).unwrap();
    let _file_operations = context
        .provide(Arc::new(FileOperationsService::new(
            Arc::clone(&database),
            context.clone(),
        )))
        .unwrap();
    let _drag_drop = context
        .provide(Arc::new(DragDropService::default()))
        .unwrap();
    let _quicklook = context
        .provide(Arc::new(QuickLookService::default()))
        .unwrap();
    let _session = context
        .provide(Arc::new(SessionService::open_at(root.join("session.json"))))
        .unwrap();
    let _grouping = context
        .provide(Arc::new(fileatlas_plugin_grouping::GroupingService))
        .unwrap();
    let _version_info = context.provide(Arc::new(
        fileatlas_plugin_version_info::VersionInfoService::open_at(
            root.join("version-info.sqlite"),
        )?,
    ))?;
    let thumbnails = Arc::new(fileatlas_plugin_thumbnails::ThumbnailService::open_at(
        root.join("thumbs"),
    )?);
    let _thumbnails = context.provide(Arc::clone(&thumbnails))?;
    let _indexing = context.provide(Arc::new(fileatlas_plugin_indexing::IndexingService::new(
        Arc::clone(&database),
        thumbnails,
    )))?;
    let _workspaces = context.provide(Arc::new(
        fileatlas_plugin_workspaces::WorkspaceService::open_at(root.join("workspaces.json"))?,
    ))?;
    let columns_plugin = fileatlas_plugin_columns::ColumnsPlugin;
    let columns_input = columns_plugin.prepare(())?;
    let columns_fiber = context
        .spawn(PreparedPlugin::from_input(columns_plugin, columns_input))
        .await?;
    columns_fiber.ready().await?;
    let versions_plugin = fileatlas_plugin_versions::VersionsPlugin;
    let versions_input = versions_plugin.prepare(fileatlas_plugin_versions::VersionsConfig {
        root: root.join("versions"),
    })?;
    let versions_fiber = context
        .spawn(PreparedPlugin::from_input(versions_plugin, versions_input))
        .await?;
    versions_fiber.ready().await?;
    let plugin = FileManagerPlugin;
    let input = plugin.prepare(FileManagerConfig).unwrap();
    let fiber = context
        .spawn(PreparedPlugin::from_input(plugin, input))
        .await
        .unwrap();
    fiber.ready().await.unwrap();
    assert_eq!(catalog.snapshot().unwrap().len(), 3);
    context
        .emit::<FolderSelectionChanged>(Routing::Unscoped, Some(42))
        .await
        .unwrap();
    let selection = context.try_service::<SelectedFolder>().unwrap();
    assert_eq!(*selection.0.lock().unwrap(), Some(42));
    context
        .emit::<TagSelectionChanged>(Routing::Unscoped, Some(8))
        .await
        .unwrap();
    let selected_tag = context.try_service::<SelectedTag>().unwrap();
    assert_eq!(selected_tag.0.lock().unwrap().included, vec![8]);
    assert_eq!(
        *context
            .try_service::<SelectedFolder>()
            .unwrap()
            .0
            .lock()
            .unwrap(),
        None
    );
    context
        .emit::<DirectoryPathSelected>(Routing::Unscoped, Some("/library/trips".into()))
        .await
        .unwrap();
    assert_eq!(
        context
            .try_service::<SelectedPath>()
            .unwrap()
            .0
            .lock()
            .unwrap()
            .as_deref(),
        Some("/library/trips")
    );
    context
        .emit::<MediaIndexChanged>(Routing::Unscoped, ())
        .await
        .unwrap();
    assert_eq!(
        context
            .try_service::<IndexRevision>()
            .unwrap()
            .0
            .load(Ordering::Acquire),
        1
    );
    fiber.dispose().await.unwrap();
    assert!(catalog.snapshot().unwrap().is_empty());
    versions_fiber.dispose().await?;
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
