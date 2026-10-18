use super::*;

#[test]
fn launcher_metadata_upgrades_old_database_and_preserves_user_edits_across_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "photag-launcher-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    let path = root.join("tags.db");
    // Reproduce the earlier launcher schema before the metadata table existed.
    let old = db::open_db_at(&path)?;
    old.execute_batch("DROP TABLE app_launcher_metadata;
        CREATE TABLE app_launcher_metadata(entry_id INTEGER PRIMARY KEY REFERENCES app_launcher_entries(id) ON DELETE CASCADE, installed_on TEXT, size_bytes INTEGER);
        INSERT INTO app_launcher_categories(id,name,position) VALUES(1,'General',0),(2,'My tools',1);
        INSERT INTO app_launcher_entries(id,discovery_key,display_name,launch_path,category_id,hidden,custom_name,custom_category)
        VALUES(1,'existing','My display name','C:\\program.exe',2,1,1,1);")?;
    drop(old);
    let database = DatabaseService::open_browser_at(&path)?;
    let mut program = LauncherDiscoveredProgram {
        discovery_key: "existing".into(),
        display_name: "Discovered name".into(),
        launch_path: "C:\\program.exe".into(),
        uninstall_command: None,
        category_name: "Development".into(),
        source: "installed".into(),
        installed_on: Some("2025-03-12".into()),
        size_bytes: Some(12345678),
    };
    database.launcher_merge_discovered(&[program.clone()])?;
    drop(database);
    let database = DatabaseService::open_browser_at(&path)?;
    let (_, entries) = database.launcher_snapshot()?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].display_name, "My display name");
    assert_eq!(entries[0].category_id, 2);
    assert!(entries[0].hidden);
    assert_eq!(entries[0].installed_on.as_deref(), Some("2025-03-12"));
    assert_eq!(entries[0].size_bytes, Some(12345678));
    assert_eq!(entries[0].pin_scope, 0);
    database.launcher_update_entry(1, "Edited in inspector", 1, false)?;
    database.launcher_set_pin_scope(1, 1)?;
    program.installed_on = None;
    program.size_bytes = None;
    database.launcher_merge_discovered(&[program])?;
    assert!(
        database
            .launcher_update_entry(1, "invalid", 999, false)
            .is_err()
    );
    drop(database);
    let database = DatabaseService::open_browser_at(&path)?;
    let (_, entries) = database.launcher_snapshot()?;
    assert_eq!(entries[0].display_name, "Edited in inspector");
    assert_eq!(entries[0].category_id, 1);
    assert!(!entries[0].hidden);
    assert_eq!(entries[0].installed_on, None);
    assert_eq!(entries[0].pin_scope, 1);
    database.launcher_set_pin_scope(1, 2)?;
    assert!(database.launcher_set_pin_scope(1, 3).is_err());
    assert_eq!(database.launcher_snapshot()?.1[0].pin_scope, 2);
    database.launcher_remove_entry(1)?;
    drop(database);
    let database = DatabaseService::open_browser_at(&path)?;
    assert!(database.launcher_snapshot()?.1.is_empty());
    database.with_reader(|conn| {
        assert_eq!(
            conn.query_row("SELECT count(*) FROM app_launcher_metadata", [], |r| r
                .get::<_, i64>(0))?,
            0
        );
        Ok(())
    })?;
    drop(database);
    for file in ["tags.db", "tags.db-wal", "tags.db-shm"] {
        let file = root.join(file);
        if file.exists() {
            std::fs::remove_file(file)?;
        }
    }
    std::fs::remove_dir(root)?;
    Ok(())
}
