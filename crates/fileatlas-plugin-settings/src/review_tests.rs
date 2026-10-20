use super::*;
fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "fileatlas-settings-review-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
#[test]
fn failed_write_keeps_the_last_saved_settings() {
    let root = root();
    let file = root.join("settings.json");
    std::fs::create_dir_all(&file).unwrap();
    let service = SettingsService::open_at(&file);
    let old = service.snapshot().unwrap().accent_name;
    assert!(
        service
            .update(|settings| settings.accent_name = "unsaved".into())
            .is_err()
    );
    assert_eq!(service.snapshot().unwrap().accent_name, old);
    let mut changed = service.snapshot().unwrap();
    changed.accent_name = "also unsaved".into();
    assert!(service.replace(changed).is_err());
    assert_eq!(service.snapshot().unwrap().accent_name, old);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn sidebar_updates_reject_stale_delivery_and_allow_retry_after_failure() {
    let root = root();
    let file = root.join("settings.json");
    std::fs::create_dir_all(&file).unwrap();
    let service = SettingsService::open_at(&file);
    let last = std::sync::Mutex::new(0);
    let update = |revision, show| fileatlas_contracts::SidebarFoldersUpdate {
        revision,
        config: fileatlas_contracts::SidebarFoldersConfig {
            show_network_drives: show,
            ..Default::default()
        },
    };
    assert!(
        service
            .apply_sidebar_update(&last, update(8, false))
            .is_err()
    );
    assert_eq!(*last.lock().unwrap(), 0);
    std::fs::remove_dir(&file).unwrap();
    service
        .apply_sidebar_update(&last, update(8, false))
        .unwrap();
    service
        .apply_sidebar_update(&last, update(3, true))
        .unwrap();
    assert!(
        !service
            .snapshot()
            .unwrap()
            .sidebar_folders
            .show_network_drives
    );
    assert_eq!(*last.lock().unwrap(), 8);
    assert!(
        !SettingsService::open_at(&file)
            .snapshot()
            .unwrap()
            .sidebar_folders
            .show_network_drives
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn concurrent_settings_edits_leave_disk_and_memory_in_agreement() {
    let root = root();
    let file = root.join("settings.json");
    let service = Arc::new(SettingsService::open_at(&file));
    let workers = (0..4)
        .map(|worker| {
            let service = service.clone();
            std::thread::spawn(move || {
                for index in 0..20 {
                    service
                        .update(|settings| {
                            settings.sidebar_folders.quick_access.push(
                                fileatlas_contracts::QuickAccessEntry {
                                    path: format!("{worker}/{index}"),
                                    display_name: None,
                                },
                            )
                        })
                        .unwrap();
                }
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().unwrap();
    }
    let memory = service.snapshot().unwrap();
    let disk = SettingsService::open_at(&file).snapshot().unwrap();
    assert_eq!(memory.sidebar_folders.quick_access.len(), 80);
    assert_eq!(memory.sidebar_folders, disk.sidebar_folders);
    std::fs::remove_dir_all(root).unwrap();
}
