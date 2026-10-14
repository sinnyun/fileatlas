use super::*;
fn skeleton(id: i64, date: i64) -> MediaSkeleton {
    MediaSkeleton {
        id,
        effective_date: date,
        is_video: false,
        is_audio: false,
    }
}
fn field(id: i64, path: &str) -> MediaGroupingFields {
    MediaGroupingFields {
        id,
        full_path: path.into(),
        modified_at: 1_700_000_000,
        root_path: "C:/library".into(),
    }
}
#[test]
fn types_are_contiguous_case_insensitive_and_keep_query_order() {
    let mut items = vec![
        skeleton(1, 0),
        skeleton(2, 0),
        skeleton(3, 0),
        skeleton(4, 0),
    ];
    let labels = GroupingService.arrange(
        &mut items,
        &[
            field(1, "C:/library/a.PNG"),
            field(2, "C:/library/b.pdf"),
            field(3, "C:/library/c.png"),
            field(4, "C:/library/README"),
        ],
        GroupingMode::FileType,
        SortOrder::NameDesc,
    );
    assert_eq!(
        items.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![4, 2, 1, 3]
    );
    assert_eq!(labels[2], Some("PNG".into()));
    let rows = GroupingService::rows(&labels, 3, &HashSet::from(["PNG".into()]));
    assert_eq!(rows.len(), 3);
    assert!(rows[2].collapsed);
    assert_eq!(rows[2].month_count, 2);
    assert_eq!(rows[2].media_range, 2..4);
}
#[test]
fn dates_group_even_when_query_sort_is_name_and_unknown_is_last() {
    assert_eq!(
        restore_collapsed(
            &HashSet::from(["January 2026".into(), "Unknown date".into()]),
            GroupingMode::Month
        ),
        HashSet::from(["2026-01".into(), "Unknown date".into()])
    );
    let mut items = vec![
        skeleton(1, 0),
        skeleton(2, 1_700_000_000),
        skeleton(3, 1_600_000_000),
    ];
    let labels = GroupingService.arrange(&mut items, &[], GroupingMode::Month, SortOrder::NameAsc);
    assert_eq!(
        items.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![2, 3, 1]
    );
    assert_eq!(labels[2], Some("Unknown date".into()));
    GroupingService.arrange(&mut items, &[], GroupingMode::Month, SortOrder::OldestFirst);
    assert_eq!(
        items.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![3, 2, 1]
    );
}
#[test]
fn roots_and_actual_parent_paths_are_distinct_and_creation_has_no_fallback() {
    let fields = [
        field(1, "C:/library/sub/a.png"),
        field(2, "C:/library/b.png"),
    ];
    let mut items = vec![skeleton(1, 1_700_000_000), skeleton(2, 1_700_000_000)];
    let roots = GroupingService.arrange(
        &mut items,
        &fields,
        GroupingMode::Folder,
        SortOrder::NameAsc,
    );
    assert_eq!(roots[0], roots[1]);
    let parents = GroupingService.arrange(
        &mut items,
        &fields,
        GroupingMode::Subfolder,
        SortOrder::NameAsc,
    );
    assert_ne!(parents[0], parents[1]);
    let created = GroupingService.arrange(
        &mut items,
        &fields,
        GroupingMode::Created,
        SortOrder::NameAsc,
    );
    assert!(created.iter().all(|v| v.as_deref() == Some("Unknown date")));
    let none = GroupingService.arrange(&mut items, &[], GroupingMode::None, SortOrder::NameAsc);
    assert!(none.iter().all(Option::is_none));
    assert_eq!(GroupingService::rows(&none, 0, &HashSet::new()).len(), 2);
}
#[tokio::test]
async fn cordis_disposal_removes_service_and_contributions() {
    use cordis::PreparedPlugin;
    let context = Context::new();
    let catalog = Arc::new(photag_ui_bridge::UiCatalog::default());
    let _ui = context
        .provide(Arc::new(photag_ui_bridge::UiBridgeService::new(
            catalog.clone(),
        )))
        .unwrap();
    let fiber = context
        .spawn(PreparedPlugin::from_input(GroupingPlugin, ()))
        .await
        .unwrap();
    fiber.ready().await.unwrap();
    assert!(context.try_service::<GroupingService>().is_ok());
    assert_eq!(catalog.snapshot().unwrap().len(), 1);
    fiber.dispose().await.unwrap();
    assert!(context.try_service::<GroupingService>().is_err());
    assert!(catalog.snapshot().unwrap().is_empty());
}

#[test]
fn subfolder_groups_put_all_roots_before_children_and_keep_file_sort() {
    let mut a = field(1, "C:/aaa/child/first.txt");
    a.root_path = "C:/aaa".into();
    let mut b = field(2, "C:/zzz/root-last.txt");
    b.root_path = "C:/zzz".into();
    let mut c = field(3, "C:/aaa/root-first.txt");
    c.root_path = "C:/aaa".into();
    let mut d = field(4, "C:/zzz/root-first.txt");
    d.root_path = "C:/zzz".into();
    let mut items = vec![
        skeleton(1, 0),
        skeleton(2, 0),
        skeleton(3, 0),
        skeleton(4, 0),
    ];
    let labels = GroupingService.arrange(
        &mut items,
        &[a, b, c, d],
        GroupingMode::Subfolder,
        SortOrder::NameDesc,
    );
    assert_eq!(
        items.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![3, 2, 4, 1]
    );
    assert_eq!(labels[0].as_deref(), Some("C:/aaa"));
    assert_eq!(labels[1], labels[2]);
    assert_eq!(labels[3].as_deref(), Some("C:/aaa/child"));
}
#[test]
fn selected_subdirectory_is_the_current_root_even_with_extended_paths() {
    let mut items = vec![skeleton(1, 0), skeleton(2, 0)];
    let fields = [
        field(1, "C:/library/nested/child/a.txt"),
        field(2, "C:/library/nested/root.txt"),
    ];
    let labels = GroupingService.arrange_in_scope(
        &mut items,
        &fields,
        GroupingMode::Subfolder,
        SortOrder::NameDesc,
        Some("C:/library/nested/"),
    );
    assert_eq!(items[0].id, 2);
    assert_eq!(labels[0].as_deref(), Some("C:/library/nested"));
    assert_eq!(
        path_key("//?/C:/library/nested/"),
        path_key("C:/library/nested")
    );
    assert_eq!(
        path_key("//?/UNC/server/share/"),
        path_key("//server/share")
    );
    // A filtered result with no direct files must not invent an empty group.
    let mut items = vec![skeleton(1, 0)];
    let labels = GroupingService.arrange_in_scope(
        &mut items,
        &fields,
        GroupingMode::Subfolder,
        SortOrder::NameAsc,
        Some("C:/library/nested"),
    );
    assert_eq!(labels, vec![Some("C:/library/nested/child".into())]);
}
