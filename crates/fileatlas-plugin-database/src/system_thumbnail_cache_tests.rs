use std::path::PathBuf;

fn temporary_root(label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(std::env::temp_dir().join(format!(
        "fileatlas-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    )))
}

#[test]
fn invalidation_is_limited_to_shell_cache_and_optional_media_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let root = temporary_root("thumbnail-invalidation")?;
    let database = crate::DatabaseService::open_at(root.join("db.sqlite"))?;
    let cache = root.join("thumbs");
    let folder = database.add_folder("C:\\photos")?;
    let shell = database.insert_media(
        folder.id,
        "shell.jpg",
        "C:\\photos\\shell.jpg",
        "image",
        1,
        1,
        Some(10),
        Some(10),
        Some(&cache.join("system-shell\\old.png").to_string_lossy()),
    )?;
    let project = database.insert_media(
        folder.id,
        "board.jpg",
        "C:\\photos\\board.jpg",
        "image",
        1,
        1,
        None,
        None,
        Some(&cache.join("boards\\preview.png").to_string_lossy()),
    )?;
    let outside_scope = database.insert_media(
        folder.id,
        "other.jpg",
        "C:\\photos\\other.jpg",
        "image",
        1,
        1,
        None,
        None,
        Some(&cache.join("system-shell\\other.png").to_string_lossy()),
    )?;

    let cleared = database.invalidate_system_shell_thumbnails(&cache, Some(&[shell]))?;
    assert_eq!(cleared.len(), 1);
    assert_eq!(
        database
            .get_media_by_id(shell)?
            .ok_or_else(|| std::io::Error::other("shell media row disappeared"))?
            .thumb_path,
        None
    );
    assert!(
        database
            .get_media_by_id(project)?
            .ok_or_else(|| std::io::Error::other("project media row disappeared"))?
            .thumb_path
            .ok_or_else(|| std::io::Error::other("project thumbnail reference disappeared"))?
            .contains("boards")
    );
    assert!(
        database
            .get_media_by_id(outside_scope)?
            .ok_or_else(|| std::io::Error::other("other media row disappeared"))?
            .thumb_path
            .is_some()
    );

    let cleared = database.invalidate_system_shell_thumbnails(&cache, None)?;
    assert_eq!(cleared.len(), 1);
    assert!(
        database
            .get_media_by_id(project)?
            .ok_or_else(|| std::io::Error::other("project media row disappeared"))?
            .thumb_path
            .is_some()
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
