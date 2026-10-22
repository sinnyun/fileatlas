use crate::*;
#[test]
fn uncached_catalogue_and_preview_reads_stay_offline() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(root.path().join("data"))?);
    let project = projects.create_project("Offline reads")?;
    let native = root.path().join("existing.vectorcraft");
    std::fs::write(&native, b"native")?;
    projects.set_document_path(&project.id, &native)?;
    let service = WorkCrateBoardsService::new(projects);
    service.configure(VectorCraftConfig {
        executable: "unavailable-vectorcraft.exe".into(),
        cli_executable: String::new(),
        port: unused_local_port(),
    })?;

    assert!(service.board_catalog(&project.id)?.is_empty());
    let board_id = format!(
        "{}:{}:42",
        project.id,
        service.projects.document_binding(&project.id)?
    );
    assert!(matches!(
        service.render_preview(&board_id, 1600),
        Err(BoardsError::PreviewUnavailable(_))
    ));
    Ok(())
}

#[test]
fn cached_catalogue_and_preview_survive_restart_without_editor()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let data = root.path().join("data");
    let projects = Arc::new(WorkCrateProjectsService::open_at(&data)?);
    let project = projects.create_project("Persistent cache")?;
    let native_path = root.path().join("document.vectorcraft");
    std::fs::write(&native_path, b"native")?;
    projects.set_document_path(&project.id, &native_path)?;
    let binding = projects.document_binding(&project.id)?;
    let service = WorkCrateBoardsService::new(projects.clone());
    let board = WorkCrateBoard {
        id: format!("{}:{binding}:12", project.id),
        project_id: project.id.clone(),
        native_id: 12,
        index: 0,
        title: "Cached page".into(),
        width_pt: 420.,
        height_pt: 297.,
        info: PageInfo {
            project_name_override: Some("旧的明确局部名称".into()),
            ..Default::default()
        },
    };
    let preview_dir = projects
        .project_directory(&project.id)?
        .join("native-previews");
    std::fs::create_dir_all(&preview_dir)?;
    let preview_path = preview_dir.join("cached.png");
    std::fs::write(&preview_path, b"png")?;
    service.write_cache(
        &project.id,
        &BoardCache {
            version: 1,
            binding: binding.clone(),
            boards: vec![board.clone()],
            previews: BTreeMap::from([(
                format!("{}@1600", board.id),
                "native-previews/cached.png".into(),
            )]),
            source: None,
        },
    )?;
    drop(service);
    drop(projects);

    let reopened = Arc::new(WorkCrateProjectsService::open_at(&data)?);
    let offline = WorkCrateBoardsService::new(reopened.clone());
    let cached = offline.board_catalog(&project.id)?;
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].id, board.id);
    assert_eq!(cached[0].title, "页面01");
    assert_eq!(cached[0].info.material_category, "旧的明确局部名称");
    assert_eq!(cached[0].info.project_name_override, None);
    reopened.rename_project(&project.id, "Title changed")?;
    assert_eq!(offline.project_name(&cached[0])?, "旧的明确局部名称");
    let mut empty = cached[0].clone();
    empty.info.material_category.clear();
    assert_eq!(offline.project_name(&empty)?, "");
    assert_eq!(
        offline.render_preview(&board.id, 1600)?,
        std::fs::canonicalize(preview_path)?
    );
    reopened.set_document_path(&project.id, &native_path)?;
    assert!(
        offline.board_catalog(&project.id)?.is_empty(),
        "a new association invalidates stale IDs"
    );
    assert!(matches!(
        offline.render_preview(&board.id, 1600),
        Err(BoardsError::PreviewUnavailable(_))
    ));
    Ok(())
}

#[test]
fn preview_state_tracks_external_saves_without_an_official_session()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(root.path().join("data"))?);
    let project = projects.create_project("External saves")?;
    let native = root.path().join("document.vectorcraft");
    std::fs::write(&native, b"first")?;
    projects.set_document_path(&project.id, &native)?;
    let binding = projects.document_binding(&project.id)?;
    let service = WorkCrateBoardsService::new(projects);
    let stamp = crate::autosync::stamp_of(&native).ok_or("written document has no stamp")?;
    let cache = |source| BoardCache {
        version: 1,
        binding: binding.clone(),
        boards: vec![],
        previews: BTreeMap::new(),
        source,
    };
    service.write_cache(&project.id, &cache(Some(stamp)))?;
    assert!(!service.preview_state(&project.id)?.stale());

    std::fs::write(&native, b"a longer externally saved document")?;
    let changed = service.preview_state(&project.id)?;
    assert!(changed.stale());
    assert_ne!(changed.stamp, changed.cache_stamp);

    service.write_cache(&project.id, &cache(None))?;
    assert!(
        service.preview_state(&project.id)?.stale(),
        "a cache without a source stamp is refreshed once"
    );

    std::fs::remove_file(&native)?;
    assert!(
        !service.preview_state(&project.id)?.stale(),
        "a missing document has nothing to sample"
    );
    Ok(())
}

fn unused_local_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}
#[test]
#[ignore = "requires the official VectorCraft CLI and WORKCRATE_NATIVE_TEST_ROOT"]
fn official_native_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var("WORKCRATE_NATIVE_TEST_ROOT")?);
    let projects = Arc::new(WorkCrateProjectsService::open_at(root.clone())?);
    let p = projects.create_project(&format!(
        "Native {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    ))?;
    let service = WorkCrateBoardsService::new(projects.clone());
    // Never hand the test document to whatever official GUI the user happens to be running:
    // an unlisted port keeps this round trip on the silent headless session it verifies.
    service.configure(VectorCraftConfig {
        executable: std::env::var("FILEATLAS_VECTORCRAFT_EXE").unwrap_or_default(),
        cli_executable: String::new(),
        port: unused_local_port(),
    })?;
    let b = service
        .create_project_document(&p.id, &root.join("documents"))?
        .into_iter()
        .next()
        .ok_or("created native document has no artboard")?;
    assert_eq!(service.board_catalog(&p.id)?.len(), 1);
    let mut info = b.info.clone();
    info.material = "中文材料123".into();
    info.material_category = "独立类别".into();
    info.quantity = "20套".into();
    info.extra.insert("工艺".into(), "切割".into());
    service.update_info(&b.id, "页面01", info)?;
    let read = service.board(&b.id)?;
    assert_eq!(read.title, "页面01");
    assert_eq!(read.info.material, "中文材料123");
    assert_eq!(read.info.material_category, "独立类别");
    assert_eq!(read.info.extra["工艺"], "切割");
    let doc = service
        .document_path(&p.id)?
        .ok_or("native document missing")?;
    let native_session = || -> Result<mcp::McpSession, BoardsError> {
        let mut c = service.headless_client(&[doc.clone()], &[doc.clone()])?;
        c.open_file(&doc)?;
        Ok(c)
    };
    let fields = native::header_fields(&mut native_session()?)?;
    let aggregate = fields[&b.native_id]["details"].id;
    let category = fields[&b.native_id]["project_name"].id;
    let before = native_session()?.execute("document.node", json!({"id":aggregate}))?;
    let category_before = native_session()?.execute("document.node", json!({"id":category}))?;
    let owned = native::owned(&mut native_session()?, b.native_id)?;
    assert_eq!(owned.layout_version, 2);
    assert_eq!(owned.fields.len(), 3);
    assert!(owned.band.is_some() && owned.chip.is_some());
    let mut edited = read.info;
    edited.extra.insert("AAA".into(), "added".into());
    service.update_info(&b.id, "页面01", edited)?;
    let after = native_session()?.execute("document.node", json!({"id":aggregate}))?;
    assert_eq!(
        before["kind"]["xf"], after["kind"]["xf"],
        "the right point-text anchor stays fixed"
    );
    assert_eq!(
        category_before,
        native_session()?.execute("document.node", json!({"id":category}))?
    );
    let grown = native::owned(&mut native_session()?, b.native_id)?;
    assert_eq!(grown.fields.len(), 3);
    assert_eq!(grown.band, owned.band);
    assert_eq!(grown.chip, owned.chip);
    assert_eq!(grown.page_group, owned.page_group);
    assert_eq!(
        grown.fields["details"].note["details"]["extra"]["AAA"],
        "added"
    );
    let mut edited = service.board(&b.id)?.info;
    edited.extra.remove("AAA");
    service.update_info(&b.id, "页面01", edited)?;
    let restored = native::owned(&mut native_session()?, b.native_id)?;
    assert_eq!(restored.fields["details"].id, aggregate);
    assert!(
        restored.fields["details"].note["details"]["extra"]
            .get("AAA")
            .is_none()
    );
    let second = service.create_page(&p.id)?;
    assert_ne!(second.native_id, b.native_id);
    assert_eq!(service.board_catalog(&p.id)?.len(), 2);
    let preview = service.render_preview(&b.id, 640)?;
    assert!(preview.is_file());
    // The native document links the placed file in place, so the reference must be a
    // stable file, not a cache preview that a later refresh garbage-collects.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root missing")?;
    let fixture = workspace_root
        .join("test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg");
    let placed_source = root.join("documents").join("placed-reference.jpeg");
    std::fs::copy(&fixture, &placed_source)?;
    let reference = projects
        .add_references(&p.id, std::slice::from_ref(&placed_source))?
        .into_iter()
        .next()
        .ok_or("saved reference missing")?;
    service.place_reference(&b.id, &reference.id, 24., 150., 200., false)?;
    let placed = native_session()?.execute("document.find", json!({"kind":"Image","limit":10}))?;
    assert!(
        placed["total"].as_u64().is_some_and(|n| n > 0),
        "official placement must create native image"
    );
    let out = projects.project_directory(&p.id)?;
    service.export_native(&[b.id.clone(), second.id], &out.join("pages.pdf"), "pdf")?;
    service.export_native(&[b.id.clone()], &out.join("page.png"), "png")?;
    service.save_project(&p.id)?;
    let original = service.document_path(&p.id)?.ok_or("native path missing")?;
    let replacement = out.join("replacement.vectorcraft");
    std::fs::copy(original, &replacement)?;
    service.attach_document(&p.id, &replacement)?;
    assert!(
        service
            .update_info(&b.id, "stale mutation", PageInfo::default())
            .is_err(),
        "old document binding must not modify copied replacement"
    );
    let rebound = service
        .refresh_project(&p.id)?
        .into_iter()
        .next()
        .ok_or("replacement board missing")?;
    service.attach_document(&p.id, &replacement)?;
    assert!(
        service
            .update_info(&rebound.id, "stale same path", PageInfo::default())
            .is_err(),
        "same-path reattach must invalidate old binding"
    );
    Ok(())
}

#[test]
#[ignore = "requires the official VectorCraft CLI and WORKCRATE_NATIVE_TEST_ROOT"]
fn command_error_keeps_the_headless_session_usable() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var("WORKCRATE_NATIVE_TEST_ROOT")?);
    let sandbox = root.join("session-survival");
    std::fs::create_dir_all(&sandbox)?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(root.clone())?);
    let service = WorkCrateBoardsService::new(projects);
    let roots = [sandbox.clone()];
    let mut c = service.headless_client(&roots, &roots)?;
    c.execute(
        "file.new",
        json!({"width":300,"height":200,"units":"Points","name":"session"}),
    )?;
    let rejected = c.execute("document.notACommand", json!({}));
    assert!(
        matches!(rejected, Err(BoardsError::Remote(_))),
        "rejected command: {rejected:?}"
    );
    let alive = c.execute("document.inspect", json!({"depth": 0}))?;
    assert_eq!(alive["title"], "session");
    Ok(())
}

#[test]
#[ignore = "requires the official VectorCraft CLI and WORKCRATE_NATIVE_TEST_ROOT"]
fn page_switches_reuse_the_same_official_session() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var("WORKCRATE_NATIVE_TEST_ROOT")?);
    let documents = root.join("page-switch-documents");
    std::fs::create_dir_all(&documents)?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(root.clone())?);
    let project = projects.create_project(&format!(
        "Pages {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    ))?;
    let service = WorkCrateBoardsService::new(projects.clone());
    service.configure(VectorCraftConfig {
        executable: std::env::var("FILEATLAS_VECTORCRAFT_EXE").unwrap_or_default(),
        cli_executable: String::new(),
        port: unused_local_port(),
    })?;
    let first = service
        .create_project_document(&project.id, &documents)?
        .into_iter()
        .next()
        .ok_or("created native document has no artboard")?;
    let second = service.create_page(&project.id)?;

    let opened = service.canvas_open(&first.id)?;
    assert_eq!(opened.board_id, first.id);
    let session = opened.session_id;
    assert!(session > 0, "every canvas process needs an identity");
    let first_preview = opened.preview.clone();

    let switched = service.canvas_open(&second.id)?;
    assert_eq!(
        switched.session_id, session,
        "a page switch inside one document must keep the official connection"
    );
    assert_eq!(switched.board_id, second.id);
    assert_ne!(
        switched.preview, first_preview,
        "each page renders its own frame"
    );
    assert!(
        switched.selection.is_empty(),
        "the page being left must not keep its selection"
    );

    let centre = [
        switched.artboard.x + switched.artboard.width / 2.,
        switched.artboard.y + switched.artboard.height / 2.,
    ];
    let created = service.canvas_create_text(centre[0], centre[1], "第二页文字", 24.)?;
    assert_eq!(created.session_id, session, "edits run in the same session");
    assert_eq!(created.board_id, second.id);

    let back = service.canvas_open(&first.id)?;
    assert_eq!(back.session_id, session, "switching back also reuses it");
    service.canvas_commit()?;
    service.canvas_close()?;

    let reopened = service.canvas_open(&first.id)?;
    assert_ne!(
        reopened.session_id, session,
        "leaving the editor releases the process, so a later open starts a new one"
    );
    service.canvas_close()?;
    Ok(())
}

#[test]
#[ignore = "requires the official VectorCraft CLI and WORKCRATE_NATIVE_TEST_ROOT"]
fn canvas_edits_land_in_the_native_document() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let root = PathBuf::from(std::env::var("WORKCRATE_NATIVE_TEST_ROOT")?);
    let documents = root.join("canvas-documents");
    std::fs::create_dir_all(&documents)?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(root.clone())?);
    let project = projects.create_project(&format!(
        "Canvas {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    ))?;
    let service = WorkCrateBoardsService::new(projects.clone());
    service.configure(VectorCraftConfig {
        executable: std::env::var("FILEATLAS_VECTORCRAFT_EXE").unwrap_or_default(),
        cli_executable: String::new(),
        port: unused_local_port(),
    })?;
    let board = service
        .create_project_document(&project.id, &documents)?
        .into_iter()
        .next()
        .ok_or("created native document has no artboard")?;
    let document = service
        .document_path(&project.id)?
        .ok_or("native document missing")?;
    let bytes_before = std::fs::read(&document)?;

    let opened = service.canvas_open(&board.id)?;
    assert!(
        !opened.live,
        "an unused control port must route to the silent headless session"
    );
    assert!(
        opened.artboard.width > 0. && opened.artboard.height > 0.,
        "the overlay mapping needs the official artboard box"
    );
    assert!(
        opened
            .preview
            .as_deref()
            .is_some_and(|path| Path::new(path).is_file()),
        "opening a page must render the official bitmap"
    );
    assert!(!opened.dirty, "a freshly opened page is saved");

    let centre = [
        opened.artboard.x + opened.artboard.width / 2.,
        opened.artboard.y + opened.artboard.height / 2.,
    ];
    let created = service.canvas_create_text(centre[0], centre[1], "画布文字", 24.)?;
    assert!(
        created.revision > opened.revision,
        "an official edit must advance the revision the poller watches"
    );
    assert!(!created.dirty, "an edit must persist before it returns");
    assert_eq!(
        std::fs::read(service.render_preview(&board.id, 1600)?)?,
        std::fs::read(created.preview.as_ref().ok_or("canvas preview missing")?)?,
        "passive cached preview must match the editing frame without a project refresh"
    );
    let text = *created
        .selection
        .first()
        .ok_or("the official program must select the text it created")?;
    assert_eq!(service.canvas_object(text)?.kind, "type");
    assert_eq!(service.canvas_object(text)?.text, "画布文字");

    // The official program applies smart guides to a pointer move, so the drag target is
    // placed in the middle of the page: every edge and centre of that box stays further from
    // the header band and the artboard guides than the snap distance.
    service.canvas_set_bounds(
        &[text],
        &CanvasGeometry {
            x: 600.,
            y: 300.,
            width: 200.,
            height: 50.,
        },
    )?;
    let placed_box = service
        .canvas_object(text)?
        .bounds
        .ok_or("official bounds missing")?;
    assert!(
        (placed_box.x - 600.).abs() < 1. && (placed_box.y - 300.).abs() < 1.,
        "reference 0 must put the box top-left where the overlay drew it, got {placed_box:?}"
    );

    let middle = [
        placed_box.x + placed_box.width / 2.,
        placed_box.y + placed_box.height / 2.,
    ];
    let dragged = service.canvas_gesture(json!([
        {"kind": "down", "x": middle[0], "y": middle[1]},
        {"kind": "drag", "x": middle[0] + 30., "y": middle[1] + 20.},
        {"kind": "up", "x": middle[0] + 30., "y": middle[1] + 20.},
    ]))?;
    assert!(
        dragged.selection.contains(&text),
        "the official hit test must select the object under the pointer"
    );
    let moved = service
        .canvas_object(text)?
        .bounds
        .ok_or("bounds missing")?;
    assert!(
        (moved.x - placed_box.x - 30.).abs() < 1. && (moved.y - placed_box.y - 20.).abs() < 1.,
        "a body drag must be the official Move, got {moved:?} after {placed_box:?}"
    );

    service.canvas_set_text(text, "第二行")?;
    assert_eq!(service.canvas_object(text)?.text, "第二行");
    service.canvas_undo()?;
    assert_eq!(service.canvas_object(text)?.text, "画布文字");
    let settled = service.canvas_redo()?;
    assert_eq!(service.canvas_object(text)?.text, "第二行");

    assert!(
        service.canvas_probe(settled.revision)?.is_none(),
        "an idle canvas must not repaint the overlay"
    );
    let stale = settled.revision - 1;
    assert_eq!(
        service
            .canvas_probe(stale)?
            .map(|snapshot| snapshot.revision),
        Some(settled.revision),
        "an observer still showing an older revision must be handed the new snapshot"
    );

    let link_dir = root.join("canvas-links");
    std::fs::create_dir_all(&link_dir)?;
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root missing")?;
    let fixture = workspace_root
        .join("test-data/fileatlas-fixtures/03-nested/level-one/level-two/nested-photo.jpeg");
    let source = link_dir.join("placed.jpeg");
    std::fs::copy(&fixture, &source)?;
    let placed = service.canvas_place(&source, centre, true)?;
    let image = *placed
        .selection
        .first()
        .ok_or("the official program must select the image it placed")?;
    let image_object = service.canvas_object(image)?;
    assert_eq!(image_object.kind, "image");
    assert!(image_object.linked, "a placed link stays linked");
    assert!(
        projects
            .references(&project.id)?
            .iter()
            .any(|reference| native::same_path(&reference.path, &source)),
        "placing a link must register its file with the project"
    );

    std::fs::OpenOptions::new()
        .append(true)
        .open(&source)?
        .write_all(b"\xff")?;
    let stale = service.canvas_object(image)?;
    assert_ne!(
        stale.status, "ok",
        "an edited source file must report itself as stale"
    );
    service.canvas_links_update(&[image])?;
    let refreshed = service.canvas_object(image)?;
    assert_eq!(
        refreshed.status, "ok",
        "the official refresh must clear the stale flag"
    );

    let replacement = link_dir.join("replacement.jpeg");
    std::fs::copy(&fixture, &replacement)?;
    service.canvas_links_relink(&[image], &replacement)?;
    assert_eq!(
        std::fs::canonicalize(&service.canvas_object(image)?.link_path)?,
        std::fs::canonicalize(&replacement)?,
        "relink must swap the source file of the same object"
    );

    let committed = service.canvas_commit()?;
    assert!(!committed.dirty, "a commit leaves the document clean");
    service.canvas_close()?;
    assert!(!service.canvas_is_open());
    assert!(
        service.canvas_probe(committed.revision)?.is_none(),
        "a closed canvas has nothing to probe"
    );
    assert_ne!(
        std::fs::read(&document)?,
        bytes_before,
        "every canvas edit must reach the native document"
    );

    let mut outside = service.headless_client(&[document.clone()], &[document.clone()])?;
    outside.open_file(&document)?;
    let texts = outside.execute("document.find", json!({"kind": "type", "limit": 10}))?;
    assert!(
        texts["total"].as_u64().is_some_and(|total| total >= 1),
        "a session that re-read the file must find the canvas text: {texts}"
    );
    let images = outside.execute("document.find", json!({"kind": "Image", "limit": 10}))?;
    assert!(
        images["total"].as_u64().is_some_and(|total| total >= 1),
        "a session that re-read the file must find the placed image: {images}"
    );

    let pages = service.refresh_project(&project.id)?;
    assert_eq!(pages.len(), 1);
    assert!(
        service.render_preview(&pages[0].id, 640)?.is_file(),
        "the cached preview must come from the saved document"
    );
    Ok(())
}
