use super::*;
use fileatlas_contracts::{FavoriteFolder, FavoriteGroup};

#[test]
fn imports_legacy_paths_once_preserves_groups_aliases_and_removal_after_restart() {
    let mut config = SidebarFoldersConfig::default();
    config.favorite_groups.push(FavoriteGroup {
        id: "work".into(),
        name: "Work".into(),
        folders: vec![FavoriteFolder {
            id: "custom".into(),
            path: r"F:\Photos".into(),
            display_name: Some("My photos".into()),
        }],
    });
    let service = SidebarFoldersService::with_config(config);
    let roots = vec![
        (1, r"f:\photos\".into(), "Photos".into()),
        (2, r"F:\Archive".into(), "Archive".into()),
    ];
    assert!(
        service
            .import_library_folders(&roots, "Favorites")
            .is_some()
    );
    let imported = service.snapshot();
    assert_eq!(imported.favorite_groups.len(), 2);
    assert_eq!(imported.favorite_groups[0].folders[0].path, r"F:\Archive");
    assert_eq!(
        imported.favorite_groups[1].folders[0]
            .display_name
            .as_deref(),
        Some("My photos")
    );
    let revision = service.revision();
    assert!(
        service
            .import_library_folders(&roots, "Favorites")
            .is_none()
    );
    assert_eq!(service.revision(), revision);
    service.update(|c| c.favorite_groups.clear());
    let Ok(json) = serde_json::to_string(&service.snapshot()) else {
        panic!("serialize sidebar");
    };
    let Ok(saved) = serde_json::from_str(&json) else {
        panic!("restore sidebar");
    };
    let restarted = SidebarFoldersService::with_config(saved);
    assert!(
        restarted
            .import_library_folders(&roots, "Favorites")
            .is_none()
    );
    assert!(restarted.snapshot().favorite_groups.is_empty());
    // SQLite folders use INTEGER PRIMARY KEY, so a removed ID can be reused.
    assert!(
        restarted
            .import_library_folders(&[(2, r"F:\New".into(), "New".into())], "Favorites")
            .is_some()
    );
    assert_eq!(
        restarted.snapshot().favorite_groups[0].folders[0].path,
        r"F:\New"
    );
}

#[test]
fn older_sidebar_json_defaults_import_markers_without_losing_data() {
    let Ok(config) = serde_json::from_str::<SidebarFoldersConfig>(
        r#"{"favorite_groups":[{"id":"g","name":"Existing","folders":[]}],"show_network_drives":false}"#,
    ) else {
        panic!("older sidebar must deserialize");
    };
    assert!(config.imported_library_paths.is_empty());
    assert_eq!(config.favorite_groups[0].name, "Existing");
    assert!(!config.show_network_drives);
}

#[test]
fn legacy_import_reuses_existing_default_named_group() {
    let mut config = SidebarFoldersConfig::default();
    config.favorite_groups.push(FavoriteGroup {
        id: "user-common".into(),
        name: "常用".into(),
        folders: vec![],
    });
    let service = SidebarFoldersService::with_config(config);
    assert!(
        service
            .import_library_folders(&[(1, r"F:\Work".into(), "Work".into())], "常用")
            .is_some()
    );
    let config = service.snapshot();
    assert_eq!(config.favorite_groups.len(), 1);
    assert_eq!(config.favorite_groups[0].id, "user-common");
    assert_eq!(config.favorite_groups[0].folders.len(), 1);
}
