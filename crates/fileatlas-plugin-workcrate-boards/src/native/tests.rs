use super::*;
fn tagged(id: u64, kind: &str, note: Value, text: Option<&str>) -> Value {
    let mut node = json!({"id":id,"attrs":{"note":note.to_string()},"kind":{"type":kind}});
    if let Some(text) = text {
        node["kind"]["runs"] = json!([{"text":text}]);
    }
    node
}
#[test]
fn actual_text_unknown_versions_and_inheritance() -> Result<(), BoardsError> {
    let value = json!({"layers":[
        tagged(1,"text",json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"material"}),Some("中文123")),
        tagged(2,"text",json!({"namespace":"fileatlas.workcrate","version":2,"board":7,"role":"material"}),Some("ignored")),
        tagged(3,"text",json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"project_name","inherited":true}),Some("全局名")),
    ]});
    let owned = extract_owned(&value)?;
    let page = &owned.boards[&7];
    assert_eq!(page.fields["material"].text, "中文123");
    assert_eq!(page.fields["material"].inherited, false);
    assert_eq!(page.fields["project_name"].text, "全局名");
    assert_eq!(page.fields["project_name"].inherited, true);
    assert_eq!(page.band, None);
    assert_eq!(owned.header_root, None);
    Ok(())
}
#[test]
fn band_chip_and_legacy_group_are_reported_separately() -> Result<(), BoardsError> {
    let band = tagged(
        4,
        "path",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"band"}),
        None,
    );
    let chip = tagged(
        6,
        "path",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"page_number_chip"}),
        None,
    );
    let legacy = tagged(
        5,
        "group",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"header"}),
        None,
    );
    let owned = extract_owned(&json!({"layers":[band,chip,legacy]}))?;
    assert_eq!(owned.boards[&7].band, Some(4));
    assert_eq!(owned.boards[&7].chip, Some(6));
    assert_eq!(owned.boards[&7].legacy_group, Some(5));
    assert!(owned.boards[&7].fields.is_empty());
    Ok(())
}
#[test]
fn page_content_groups_and_master_are_reported() -> Result<(), BoardsError> {
    let subgroup = tagged(
        4,
        "group",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"header_page"}),
        None,
    );
    let content = tagged(
        5,
        "group",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"content_root"}),
        None,
    );
    // The master wraps every page's band, so it carries no board id of its own.
    let master = tagged(
        6,
        "group",
        json!({"namespace":"fileatlas.workcrate","version":1,"role":"header_root"}),
        None,
    );
    let other = tagged(
        7,
        "group",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":8,"role":"header_page"}),
        None,
    );
    let owned = extract_owned(&json!({"layers":[
        json!({"id":9,"kind":{"type":"group","children":[subgroup]}}),
        content, master, other,
    ]}))?;
    assert_eq!(owned.boards[&7].page_group, Some(4));
    assert_eq!(owned.boards[&7].content_group, Some(5));
    assert_eq!(owned.boards[&8].page_group, Some(7));
    assert_eq!(owned.header_root, Some(6));
    Ok(())
}
#[test]
fn legacy_group_children_stay_readable() -> Result<(), BoardsError> {
    let text = tagged(
        3,
        "text",
        json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"size"}),
        Some("80 * 180cm"),
    );
    let group = json!({"id":9,"kind":{"type":"group","children":[text]}});
    let owned = extract_owned(&json!({"layers":[group]}))?;
    assert_eq!(owned.boards[&7].fields["size"].id, 3);
    Ok(())
}
#[test]
fn duplicate_roles_refuse_ambiguous_mutation() {
    let node = |id: u64| {
        tagged(
            id,
            "text",
            json!({"namespace":"fileatlas.workcrate","version":1,"board":7,"role":"material"}),
            Some("A"),
        )
    };
    assert!(extract_owned(&json!({"layers":[node(1),node(2)]})).is_err());
}
#[test]
fn click_drill_picks_the_topmost_child_under_the_pointer() {
    let boxed = |id: u64, kind: &str, x: f64, y: f64| json!({"id":id,"kind":kind,"locked":false,"bounds":{"x":x,"y":y,"width":100.,"height":50.}});
    let nested = json!({
        "id":3,"kind":"Group","locked":false,
        "children":[boxed(4,"Image",10.,10.),boxed(5,"Type",10.,10.)],
        "bounds":{"x":0.,"y":0.,"width":200.,"height":100.},
    });
    let content = json!({
        "id":2,"kind":"Group","locked":false,
        "children":[boxed(6,"Path",0.,80.),nested],
        "bounds":{"x":0.,"y":0.,"width":200.,"height":150.},
    });
    let layers = vec![boxed(1, "Path", 300., 0.), content.clone()];
    // Later children sit above earlier ones, so the text over the image wins.
    assert_eq!(deepest_at(&content, (15., 15.)), Some(5));
    // Inside the nested group's bounds but on none of its children: the group itself.
    assert_eq!(deepest_at(&content, (50., 90.)), Some(3));
    assert_eq!(deepest_at(&content, (150., 140.)), None);
    // Locked groups (the material band's master) and their children are reported so
    // the drill can deselect them instead.
    let header = json!({"id":7,"kind":"Group","locked":true,"children":[boxed(9,"Type",10.,10.)]});
    assert_eq!(
        locate(&layers, 5, false).map(|(node, locked)| (node["id"].as_u64(), locked)),
        Some((Some(5), false))
    );
    assert_eq!(
        locate(&[header], 9, false).map(|(_, locked)| locked),
        Some(true)
    );
    assert_eq!(locate(&layers, 99, false), None);
}
#[test]
fn resource_diagnostics_summarize_missing_files_and_repair_steps() {
    let report = json!({
        "title":"新建项目 1.vectorcraft",
        "missingLinks":[
            {"ids":[28],"name":"2026-10-21 20-46-04.png","path":r"\\?\F:\AI\geren\pho_cordis\docs\2026-10-21 20-46-04.png"},
            {"path":r"\\?\F:\素材\cover.jpg"},
        ],
        "modifiedLinks":[],
        "warnings":[],
    });
    let message = describe_open_report(&report, false);
    assert!(
        message.contains("资源文件缺失：2026-10-21 20-46-04.png、cover.jpg"),
        "{message}"
    );
    assert!(message.contains("关联工程"), "{message}");
    assert!(
        !message.contains("missingLinks"),
        "raw report leaked: {message}"
    );
    assert!(
        !message.contains(r"\\?\"),
        "internal path form leaked: {message}"
    );
}
#[test]
fn sandboxed_diagnostics_hide_links_that_exist_on_disk() {
    let existing = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let report = json!({
        "missingLinks":[
            {"name":"present.png","path":existing},
            {"name":"gone.png","path":r"\\?\F:\absent\gone.png"},
        ],
        "modifiedLinks":[],
        "warnings":[],
    });
    let message = describe_open_report(&report, true);
    assert!(message.contains("资源文件缺失：gone.png"), "{message}");
    assert!(
        !message.contains("present.png"),
        "a sandbox artifact must not be shown as missing: {message}"
    );
    let live = describe_open_report(&report, false);
    assert!(
        live.contains("present.png"),
        "a live report keeps every entry: {live}"
    );
}
#[test]
fn resource_diagnostics_count_link_changes_and_warnings() {
    let report = json!({
        "missingLinks":[],
        "modifiedLinks":[{"path":"a.png"}],
        "warnings":["one","two"],
    });
    let message = describe_open_report(&report, false);
    assert!(message.contains("改写了 1 处资源链接"), "{message}");
    assert!(message.contains("2 条资源警告"), "{message}");
    assert!(message.contains("关联工程"), "{message}");
}
