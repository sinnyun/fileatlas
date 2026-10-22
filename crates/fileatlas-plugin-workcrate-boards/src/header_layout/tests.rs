use super::*;

fn sizes(entries: &[(&str, f64, f64)]) -> BTreeMap<String, Measured> {
    entries
        .iter()
        .map(|(role, width, height)| {
            (
                role.to_string(),
                Measured {
                    width: *width,
                    height: *height,
                },
            )
        })
        .collect()
}

fn field(role: &'static str, label: &'static str, text: &'static str) -> Field<'static> {
    Field { role, label, text }
}

fn placement<'a>(layout: &'a Layout, role: &str) -> Option<&'a Placement> {
    layout.rows.iter().find(|p| p.role == role)
}

const ARTBOARD: (f64, f64, f64) = (0., 0., 1190.55);

/// The two-step the production driver performs: ask the official metrics whether the
/// run fits the primary row, then lay the band out with that decision.
fn build_at(
    x: f64,
    y: f64,
    width: f64,
    page_number: &str,
    project_name: &str,
    group: &[Field<'_>],
    sub: &[Field<'_>],
    measured: &BTreeMap<String, Measured>,
) -> Result<Layout, HeaderLayoutError> {
    let group_in_primary = group_fits(x, width, page_number, project_name, group, measured)?;
    build(
        x,
        y,
        width,
        page_number,
        project_name,
        group,
        sub,
        group_in_primary,
        measured,
    )
}

fn full_measured() -> BTreeMap<String, Measured> {
    // The driver sites each separator as `separator:<index>`, never as the bare role, so
    // the metrics map here holds exactly the roles the production header can provide.
    let mut measured = sizes(&[
        (ROLE_PAGE_NUMBER, 16., 16.),
        (ROLE_PROJECT_NAME, 200., 26.),
        ("material", 60., 17.),
        ("size", 70., 17.),
        ("quantity", 40., 17.),
        ("label:size_label", 44., 15.),
        ("size_label", 80., 15.),
    ]);
    for index in 0..2 {
        measured.insert(
            separator_role(index),
            Measured {
                width: 6.,
                height: 17.,
            },
        );
    }
    measured
}

fn group() -> Vec<Field<'static>> {
    vec![
        field("material", "物料", "人物立牌"),
        field("size", "尺寸", "80 * 180cm"),
        field("quantity", "数量", "20 套"),
    ]
}

#[test]
fn primary_row_keeps_the_left_pair_and_right_aligns_the_group() {
    let layout = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "1024 工程师文化节",
        &group(),
        &[],
        &full_measured(),
    )
    .expect("layout");
    assert!(layout.group_in_primary);
    assert_eq!(
        placement(&layout, ROLE_PAGE_NUMBER).map(|p| p.left),
        Some(24.)
    );
    let Some(chip) = layout.chip else {
        panic!("page number chip");
    };
    assert_eq!(chip.x, 24. - CHIP_PAD_X);
    assert_eq!(
        placement(&layout, ROLE_PROJECT_NAME).map(|p| p.left),
        Some(chip.x + chip.width + NAME_CHIP_GAP)
    );
    let right = ARTBOARD.2 - TEXT_INSET;
    assert_eq!(
        placement(&layout, "quantity").map(|p| p.left),
        Some(right - 40.)
    );
    assert!(placement(&layout, "size").is_some());
    // material · size · quantity share one centre line and never overlap.
    let order = ["material", "separator:0", "size", "separator:1", "quantity"];
    let mut previous_right = f64::MIN;
    for role in order {
        let item = placement(&layout, role).unwrap_or_else(|| panic!("missing {role}"));
        assert!(
            item.left >= previous_right,
            "{role} overlaps the item to its left: {} < {previous_right}",
            item.left
        );
        previous_right = item.left + item.width;
        assert!(item.left + item.width <= right + f64::EPSILON);
    }
    assert_eq!(previous_right, right);
    let centres = [ROLE_PAGE_NUMBER, ROLE_PROJECT_NAME, "material", "quantity"]
        .iter()
        .map(|role| {
            let item = placement(&layout, role).expect(role);
            item.top + item.height / 2.
        })
        .collect::<Vec<_>>();
    assert!(centres.iter().all(|c| (c - PRIMARY_CENTER).abs() < 1e-9));
    // No detail rows: the band stays a compact strip across the top of the page.
    assert!(
        layout.band.height < 60.,
        "band height {}",
        layout.band.height
    );
}

#[test]
fn the_page_number_sits_in_a_chip_centred_in_the_band() {
    let layout = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &full_measured(),
    )
    .expect("layout");
    let Some(chip) = layout.chip else {
        panic!("page number chip");
    };
    let Some(number) = placement(&layout, ROLE_PAGE_NUMBER) else {
        panic!("page number");
    };
    assert_eq!(
        number.left,
        chip.x + CHIP_PAD_X,
        "the number is inset inside its chip"
    );
    assert_eq!(
        number.top + number.height / 2.,
        chip.y + chip.height / 2.,
        "the number is centred inside its chip"
    );
    assert_eq!(chip.radius, CHIP_RADIUS);
    assert!(chip.y >= layout.band.y, "chip above the band");
    assert!(
        chip.y + chip.height <= layout.band.y + layout.band.height,
        "chip below the band"
    );
    assert!(
        placement(&layout, ROLE_PROJECT_NAME).is_some_and(|name| name.left > chip.x + chip.width),
        "the project name starts after the chip"
    );
}

#[test]
fn a_page_without_a_number_has_no_chip() {
    let measured = sizes(&[(ROLE_PROJECT_NAME, 100., 26.)]);
    let layout = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "",
        "项目",
        &[],
        &[],
        &measured,
    )
    .expect("layout");
    assert!(layout.chip.is_none());
    assert_eq!(
        placement(&layout, ROLE_PROJECT_NAME).map(|p| p.left),
        Some(24.)
    );
}

#[test]
fn the_band_covers_every_row_and_stays_inside_the_page() {
    let mut measured = full_measured();
    for role in ["material", "size", "quantity"] {
        measured.insert(
            label_role(role),
            Measured {
                width: 44.,
                height: 15.,
            },
        );
    }
    let layout = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "很长的项目名称",
        &group(),
        &[field("size_label", "规格", "500 × 100cm")],
        &measured,
    )
    .expect("layout");
    assert_eq!(layout.band.x, ARTBOARD.0 + BAND_INSET);
    assert_eq!(layout.band.y, ARTBOARD.1 + BAND_INSET);
    assert_eq!(layout.band.width, ARTBOARD.2 - BAND_INSET * 2.);
    if let Some(chip) = layout.chip {
        assert!(
            chip.x >= layout.band.x && chip.x + chip.width <= layout.band.x + layout.band.width
        );
        assert!(
            chip.y >= layout.band.y && chip.y + chip.height <= layout.band.y + layout.band.height
        );
    }
    for row in &layout.rows {
        assert!(row.top >= layout.band.y, "row {} above the band", row.role);
        assert!(
            row.top + row.height <= layout.band.y + layout.band.height,
            "row {} below the band",
            row.role
        );
        assert!(
            row.left >= layout.band.x && row.left + row.width <= layout.band.x + layout.band.width
        );
    }
}

#[test]
fn a_group_too_wide_for_the_primary_row_flows_into_detail_rows() {
    let mut measured = full_measured();
    measured.insert(
        ROLE_PROJECT_NAME.to_string(),
        Measured {
            width: 1080.,
            height: 26.,
        },
    );
    // Only the demoted rows need label metrics; a group that stays on the primary row
    // must lay out without them.
    for role in ["material", "size", "quantity"] {
        measured.insert(
            label_role(role),
            Measured {
                width: 44.,
                height: 15.,
            },
        );
    }
    let layout = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "很长的项目名称".repeat(20).as_str(),
        &group(),
        &[field("size_label", "规格", "500 × 100cm")],
        &measured,
    )
    .expect("layout");
    assert!(!layout.group_in_primary);
    assert!(placement(&layout, "separator:0").is_none());
    // Every demoted field keeps its own value role and gains a label row.
    for role in ["material", "size", "quantity"] {
        assert!(
            placement(&layout, role).is_some(),
            "{role} lost from the band"
        );
        assert!(
            placement(&layout, &label_role(role)).is_some(),
            "{role} has no label"
        );
    }
    let first = placement(&layout, "size_label").expect("size label row");
    let second = placement(&layout, "material").expect("material row");
    assert!(second.top > first.top, "detail rows must stack downwards");
    assert_eq!(
        placement(&layout, "quantity").map(|p| p.left),
        placement(&layout, "material").map(|p| p.left),
        "detail values line up in one column"
    );
    assert!(layout.band.height > 60., "band must grow with the rows");
}

#[test]
fn adding_a_detail_field_never_moves_the_primary_row() {
    let without = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &full_measured(),
    )
    .expect("layout");
    let mut measured = full_measured();
    measured.insert(
        "label:extra:工艺".to_string(),
        Measured {
            width: 32.,
            height: 15.,
        },
    );
    measured.insert(
        "extra:工艺".to_string(),
        Measured {
            width: 32.,
            height: 15.,
        },
    );
    let with = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[field("extra:工艺", "工艺", "切割")],
        &measured,
    )
    .expect("layout");
    for role in [
        ROLE_PAGE_NUMBER,
        ROLE_PROJECT_NAME,
        "material",
        "size",
        "quantity",
        "separator:0",
        "separator:1",
    ] {
        assert_eq!(
            placement(&without, role).map(|p| (p.left, p.top)),
            placement(&with, role).map(|p| (p.left, p.top)),
            "{role} moved when a detail field was added"
        );
    }
    assert!(with.band.height > without.band.height);
    // Removing the field again restores the original band exactly.
    let restored = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &full_measured(),
    )
    .expect("layout");
    assert_eq!(restored, without);
}

#[test]
fn empty_fields_and_their_labels_are_left_out() {
    let measured = sizes(&[(ROLE_PAGE_NUMBER, 16., 16.), (ROLE_PROJECT_NAME, 100., 26.)]);
    let layout = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "",
        "项目",
        &[],
        &[],
        &measured,
    )
    .expect("layout");
    assert_eq!(
        layout
            .rows
            .iter()
            .map(|p| p.role.as_str())
            .collect::<Vec<_>>(),
        vec![ROLE_PROJECT_NAME]
    );
}

#[test]
fn an_unmeasured_role_is_refused_instead_of_guessed() {
    let mut measured = full_measured();
    measured.remove("size");
    let error = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &measured,
    )
    .expect_err("missing metrics must be an error");
    assert_eq!(error, HeaderLayoutError::Unmeasured("size".into()));
}

#[test]
fn each_separator_is_measured_under_its_own_index() {
    let mut measured = full_measured();
    measured.remove(&separator_role(1));
    let error = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &measured,
    )
    .expect_err("the run must not reuse one separator metric for all of them");
    assert_eq!(error, HeaderLayoutError::Unmeasured(separator_role(1)));
}

#[test]
fn text_wider_than_the_page_is_refused() {
    let mut measured = full_measured();
    for role in ["material", "size", "quantity"] {
        measured.insert(
            label_role(role),
            Measured {
                width: 44.,
                height: 15.,
            },
        );
    }
    measured.insert(
        ROLE_PROJECT_NAME.to_string(),
        Measured {
            width: 1400.,
            height: 26.,
        },
    );
    let error = build_at(
        ARTBOARD.0,
        ARTBOARD.1,
        ARTBOARD.2,
        "01",
        "超长的项目名称",
        &group(),
        &[],
        &measured,
    )
    .expect_err("the band must not print past the page");
    assert_eq!(error, HeaderLayoutError::TooWide(ROLE_PROJECT_NAME.into()));
}

#[test]
fn the_artboard_origin_shifts_every_box_with_it() {
    let at_origin = build_at(
        0.,
        0.,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &full_measured(),
    )
    .expect("layout");
    let offset = build_at(
        250.,
        120.,
        ARTBOARD.2,
        "01",
        "项目",
        &group(),
        &[],
        &full_measured(),
    )
    .expect("layout");
    for (a, b) in at_origin.rows.iter().zip(&offset.rows) {
        assert_eq!(b.left, a.left + 250.);
        assert_eq!(b.top, a.top + 120.);
    }
    assert_eq!(offset.band.height, at_origin.band.height);
    assert_eq!(offset.band.x, at_origin.band.x + 250.);
    assert_eq!(offset.band.y, at_origin.band.y + 120.);
}
