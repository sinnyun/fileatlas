use super::*;
fn directory(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/validation/profiles")
        .join(format!(
            "whiteboard-test-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_nanos())
                .unwrap_or_default()
        ))
}
#[test]
fn native_scene_metadata_and_templates_survive_restart() -> Result<()> {
    let root = directory("restart");
    let store = WhiteboardService::open(root.clone())?;
    let id = store.create("原生画板")?;
    let native = r#"{"camera":{"x":12.0,"y":-8.0,"zoom":2.0},"elements":[]}"#;
    store.save_scene(id, native.into())?;
    store.update_board(id, |b| {
        b.toolbar_vertical = true;
        b.toolbar_pos = Some((15., 20.));
    })?;
    store.save_template("模板".into(), "[]".into())?;
    store.save_colors(vec![17, 29])?;
    store.flush()?;
    assert_eq!(store.status().revision, store.status().saved_revision);
    store.stop()?;
    assert!(store.rename(id, "stale callback").is_err());
    drop(store);
    let store = WhiteboardService::open(root)?;
    assert_eq!(store.board(id)?.scene, native);
    assert!(store.board(id)?.toolbar_vertical);
    assert_eq!(store.snapshot().colors, vec![17, 29]);
    assert_eq!(store.templates().len(), 1);
    Ok(())
}
#[test]
fn bad_json_recovers_backup_but_future_versions_are_never_overwritten() -> Result<()> {
    let root = directory("recovery");
    let store = WhiteboardService::open(root.clone())?;
    store.rename(1, "first")?;
    store.flush()?;
    store.rename(1, "second")?;
    store.flush()?;
    drop(store);
    let path = root.join("library.json");
    std::fs::write(&path, "broken").map_err(|e| e.to_string())?;
    let store = WhiteboardService::open(root.clone())?;
    assert_eq!(store.board(1)?.title, "first");
    assert_eq!(
        std::fs::read_to_string(path.with_extension("json.corrupt")).map_err(|e| e.to_string())?,
        "broken"
    );
    drop(store);
    std::fs::write(&path, r#"{"version":99}"#).map_err(|e| e.to_string())?;
    assert!(WhiteboardService::open(root).is_err());
    assert_eq!(
        std::fs::read_to_string(path).map_err(|e| e.to_string())?,
        r#"{"version":99}"#
    );
    Ok(())
}
#[test]
fn import_export_preserve_source_and_reject_unrelated_json() -> Result<()> {
    let root = directory("import");
    let store = WhiteboardService::open(root.clone())?;
    let source = root.join("source.json");
    let native = Scene::default().to_json();
    std::fs::write(&source, &native).map_err(|e| e.to_string())?;
    let id = store.import(&source)?;
    store.export(id, &root.join("export.json"))?;
    assert_eq!(
        std::fs::read_to_string(&source).map_err(|e| e.to_string())?,
        native
    );
    assert_eq!(
        std::fs::read_to_string(root.join("export.json")).map_err(|e| e.to_string())?,
        native
    );
    let before = store.snapshot().boards.len();
    std::fs::write(&source, r#"{"unrelated":true}"#).map_err(|e| e.to_string())?;
    assert!(store.import(&source).is_err());
    assert_eq!(store.snapshot().boards.len(), before);
    Ok(())
}

#[test]
fn explicit_flush_retries_a_failed_write_without_losing_in_memory_edits() -> Result<()> {
    let root = directory("retry");
    let store = WhiteboardService::open(root.clone())?;
    let block = root.join("library.json.tmp");
    std::fs::create_dir(&block).map_err(|e| e.to_string())?;
    store.rename(1, "pending edit")?;
    assert!(store.flush().is_err());
    assert!(store.status().error.is_some());
    std::fs::remove_dir(&block).map_err(|e| e.to_string())?;
    store.flush()?;
    assert_eq!(store.status().error, None);
    assert_eq!(store.status().revision, store.status().saved_revision);
    drop(store);
    assert_eq!(
        WhiteboardService::open(root)?.board(1)?.title,
        "pending edit"
    );
    Ok(())
}
