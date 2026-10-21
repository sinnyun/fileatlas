use super::*;

#[test]
fn refresh_and_replace_cover_the_original_frame_for_all_source_sizes()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(dir.path().join("store"))?);
    let project = projects.create_project("Replacement proof")?;
    let path = dir.path().join("image.png");
    image::RgbImage::new(800, 400).save(&path)?;
    let reference = projects
        .add_references(&project.id, &[path.clone()])?
        .remove(0);
    let service = WorkCrateBoardsService::new(projects.clone());
    let board = service.create_page(&project.id)?;
    let board = service.place_reference(&board.id, &reference.id, 24., 80., 400., false)?;
    let asset = board.placements.keys().next().ok_or("no image")?.clone();
    let original = board.image_layout(&asset)?.frame;
    let cropped = [
        original[0] + 30.,
        original[1] + 25.,
        original[2] - 80.,
        original[3] - 35.,
    ];
    let board = service.set_image_frame(&board.id, &asset, cropped, true)?;
    for (index, (w, h)) in [(1600, 800), (300, 900), (1200, 300)]
        .into_iter()
        .enumerate()
    {
        let replacement = if index == 0 {
            path.clone()
        } else {
            dir.path().join(format!("replacement-{index}.png"))
        };
        image::RgbImage::new(w, h).save(&replacement)?;
        let updated = service.refresh_image(
            &board.id,
            &asset,
            (index > 0).then_some(replacement.as_path()),
        )?;
        let layout = updated.image_layout(&asset)?;
        assert_eq!(
            layout.frame, cropped,
            "original crop frame and position must be unchanged"
        );
        let f = layout.frame;
        let c = layout.content;
        assert!(((c[2] - c[0]) / (c[3] - c[1]) - w as f64 / h as f64).abs() < 1e-6);
        assert!(
            c[0] <= f[0] + 1e-6
                && c[1] <= f[1] + 1e-6
                && c[2] >= f[2] - 1e-6
                && c[3] >= f[3] - 1e-6
        );
        assert!(
            ((c[0] + c[2]) - (f[0] + f[2])).abs() < 1e-6
                && ((c[1] + c[3]) - (f[1] + f[3])).abs() < 1e-6
        );
    }
    Ok(())
}

#[test]
fn native_multiline_text_persists_style_renders_alpha_and_resizes_without_glyph_scaling()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(dir.path().join("store"))?);
    let project = projects.create_project("Text proof")?;
    let service = WorkCrateBoardsService::new(projects);
    let board = service.create_page(&project.id)?;
    let first = service.add_text(&board.id)?;
    let key = first.texts.keys().next().ok_or("missing text")?.clone();
    let second = service.add_text(&board.id)?;
    assert_eq!(second.texts.len(), 2);
    let edited = service.update_text(
        &board.id,
        &key,
        "第一行\n第二行",
        26.,
        "#cc2244",
        [72., 120., 432., 300.],
    )?;
    assert_eq!(edited.texts[&key].size_pt, 26.);
    assert!(serde_json::to_string(&edited.source)?.contains("Microsoft YaHei"));
    let sprite = service.render_text(&board.id, &key, 1.)?;
    assert!(sprite.pixels.chunks_exact(4).any(|p| p[3] > 0));
    assert!(
        sprite.pixels.chunks_exact(4).any(|p| p[3] == 0),
        "text sprite must be transparent above photographs"
    );
    let resized = service.move_text_frame(&board.id, &key, [100., 150., 300., 400.])?;
    assert_eq!(resized.texts[&key].size_pt, 26.);
    assert_eq!(resized.text_frame(&key)?, [100., 150., 300., 400.]);
    let loaded = service.load_board(&project.id, &board.id)?;
    assert_eq!(loaded.texts[&key].text, "第一行\n第二行");
    assert_eq!(loaded.texts[&key].color, "#cc2244");
    assert!(
        service
            .update_text(
                &board.id,
                &key,
                "invalid",
                f64::NAN,
                "#cc2244",
                [1., 1., 20., 20.]
            )
            .is_err()
    );
    assert_eq!(service.delete_text(&board.id, &key)?.texts.len(), 1);
    Ok(())
}

#[test]
fn all_native_text_options_and_selected_formats_survive_typing_and_reload()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(dir.path().join("store"))?);
    let project = projects.create_project("Native typography")?;
    let service = WorkCrateBoardsService::new(projects);
    let board = service.create_page(&project.id)?;
    let board = service.add_text(&board.id)?;
    let key = board.texts.keys().next().ok_or("no text")?.clone();
    let f = board.text_frame(&key)?;
    service.update_text(&board.id, &key, "第一段文字\n第二行", 24., "#000000", f)?;
    let catalog = service.text_options(&board.id, &key, None)?;
    assert!(
        catalog["defaults"]["chars"]
            .as_object()
            .ok_or("missing native property map")?
            .len()
            > 60
    );
    assert!(
        catalog["defaults"]["para"]
            .as_object()
            .ok_or("missing native property map")?
            .len()
            > 60
    );
    assert_eq!(
        catalog["defaults"]["frame"]
            .as_object()
            .ok_or("missing native property map")?
            .len(),
        27
    );
    assert!(catalog["defaults"]["frame"].get("path").is_some());
    assert!(
        catalog["defaults"]["frame"]
            .get("autoSizeMinHeight")
            .is_some()
    );
    service.set_text_options(
        &board.id,
        &key,
        Some([0, 6]),
        "chars",
        json!({"underline":true,"tracking":80.}),
    )?;
    service.set_text_options(
        &board.id,
        &key,
        None,
        "para",
        json!({"align":"center","spaceBefore":9.}),
    )?;
    service.set_text_options(
        &board.id,
        &key,
        None,
        "frame",
        json!({"columns":2,"gutter":16.,"inset":[4.,5.,6.,7.]}),
    )?;
    service.set_text_options(&board.id, &key, None, "openType", json!({"liga":false}))?;
    service.update_text(&board.id, &key, "第一段文字\n第二行新增", 24., "#000000", f)?;
    let first = service.text_options(&board.id, &key, Some([0, 6]))?;
    let rest = service.text_options(&board.id, &key, Some([6, 9]))?;
    assert_eq!(first["chars"]["underline"], true);
    assert_eq!(first["chars"]["tracking"], 80.);
    assert_eq!(rest["chars"]["underline"], false);
    assert_eq!(first["chars"]["ligatures"], false);
    assert_eq!(first["para"]["align"], "center");
    assert_eq!(first["frame"]["columns"], 2);
    assert!(
        service
            .set_text_options(&board.id, &key, None, "frame", json!({"columns":0}))
            .is_err()
    );
    assert!(
        service
            .set_text_options(
                &board.id,
                &key,
                Some([1, 6]),
                "chars",
                json!({"underline":false})
            )
            .is_err()
    );
    service.set_text_options(&board.id, &key, None, "chars", json!({"fill":"#ee4455"}))?;
    assert_eq!(service.board(&board.id)?.texts[&key].color, "#ee4455");
    service.update_text(&board.id, &key, "第一段文字\n第二行新增", 24., "#000000", f)?;
    assert_eq!(service.board(&board.id)?.texts[&key].color, "#000000");
    let loaded = service.load_board(&project.id, &board.id)?;
    assert_eq!(loaded.text_appearance(&key, 0)["chars"]["underline"], true);
    assert_eq!(loaded.text_appearance(&key, 9)["chars"]["underline"], false);
    Ok(())
}

#[test]
fn complete_native_attribute_catalogue_is_accepted_by_native_commands()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let projects = Arc::new(WorkCrateProjectsService::open_at(dir.path().join("store"))?);
    let project = projects.create_project("Full native catalogue")?;
    let service = WorkCrateBoardsService::new(projects);
    let board = service.create_page(&project.id)?;
    let board = service.add_text(&board.id)?;
    let key = board.texts.keys().next().ok_or("no text")?.clone();
    let catalog = service.text_options(&board.id, &key, None)?;
    for scope in ["chars", "para", "frame"] {
        service.set_text_options(
            &board.id,
            &key,
            None,
            scope,
            catalog["defaults"][scope].clone(),
        )?;
    }
    assert_eq!(
        service.text_options(&board.id, &key, None)?["frame"]["columns"],
        1
    );
    Ok(())
}
