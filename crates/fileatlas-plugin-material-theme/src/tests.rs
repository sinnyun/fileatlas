use super::*;
use fileatlas_ui_bridge::theme::{contrast_ratio, packed, parse_hex_color};
use gpui_kit::component::{Theme, ThemeSet};
fn must<T, E: std::fmt::Debug>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("{error:?}"),
    }
}
fn rgb_value([red, green, blue]: [u8; 3]) -> u32 {
    u32::from_be_bytes([0, red, green, blue])
}
#[test]
fn manual_roles_override_both_generated_modes_and_reject_unknown_roles() {
    let mut p = Preferences::default();
    p.manual_colors
        .insert("secondary.background".into(), "#aabbcc".into());
    let value: serde_json::Value = must(serde_json::from_str(&must(generate::generate(&p))));
    for theme in value["themes"]
        .as_array()
        .unwrap_or_else(|| panic!("themes missing"))
    {
        assert_eq!(theme["colors"]["secondary.background"], "#aabbcc");
        assert_eq!(theme["colors"]["button.secondary.background"], "#aabbcc");
    }
    p.manual_colors
        .insert("not.a.role".into(), "#ffffff".into());
    assert!(generate::generate(&p).is_err());
}

#[test]
fn editable_palette_is_limited_to_eight_semantic_families() {
    assert_eq!(
        PALETTE_ROLES.len() + 1,
        8,
        "seven swatches plus the primary seed"
    );
}

#[test]
fn toolbar_list_tree_and_table_hover_resolve_to_one_native_color() {
    let json = generate::generate(&Preferences::default()).unwrap();
    let set: ThemeSet = serde_json::from_str(&json).unwrap();
    for config in set.themes {
        let mut native = Theme::default();
        native.apply_config(&std::rc::Rc::new(config));
        let colors = native.colors;
        assert_eq!(packed(colors.accent), packed(colors.button_hover));
        assert_eq!(packed(colors.accent), packed(colors.list_hover));
        assert_eq!(packed(colors.accent), packed(colors.table_hover));
    }
}

#[test]
fn manual_background_is_applied_to_panel_and_modal_surfaces_in_both_modes() {
    let mut p = Preferences::default();
    p.manual_colors
        .insert("background".into(), "#ff00cc".into());
    let value: serde_json::Value = must(serde_json::from_str(&must(generate::generate(&p))));
    let themes = value["themes"]
        .as_array()
        .unwrap_or_else(|| panic!("themes missing"));
    for theme in themes {
        for role in [
            "background",
            "list.background",
            "table.background",
            "sidebar.background",
            "title_bar.background",
            "status_bar.background",
            "group_box.background",
            "accordion.background",
            "list.head.background",
            "table.head.background",
            "table.foot.background",
            "tab_bar.background",
            "tab_bar.segmented.background",
            "popover.background",
            "muted.background",
            "list.even.background",
            "table.even.background",
            "slider.background",
            "skeleton.background",
            "button.background",
            "tab.background",
            "switch.background",
            "description_list.label.background",
        ] {
            assert_eq!(theme["colors"][role], "#ff00cc", "{role}");
        }
    }

    p.manual_colors
        .insert("popover.background".into(), "#123abc".into());
    assert!(generate::generate(&p).is_err());
    p.manual_colors.remove("popover.background");
    let value: serde_json::Value = must(serde_json::from_str(&must(generate::generate(&p))));
    let set: ThemeSet = must(serde_json::from_value(value));
    for config in set.themes {
        let mut native = Theme::default();
        native.apply_config(&std::rc::Rc::new(config));
        assert_eq!(packed(native.colors.sidebar), 0xff00cc);
        assert_eq!(packed(native.colors.muted), 0xff00cc);
        assert_eq!(packed(native.colors.group_box), 0xff00cc);
        assert_eq!(packed(native.colors.popover), 0xff00cc);
        assert_eq!(packed(native.colors.button), 0xff00cc);
    }
}

#[test]
fn named_theme_config_ids_are_unique_for_registry_replacement_safety() {
    let mut first = Preferences::default();
    first.theme_name = "First-Palette".into();
    let mut second = first.clone();
    second.theme_name = "Second-Palette".into();
    let first: serde_json::Value = must(serde_json::from_str(&must(generate::generate(&first))));
    let second: serde_json::Value = must(serde_json::from_str(&must(generate::generate(&second))));
    let refreshed: serde_json::Value = must(serde_json::from_str(&must(generate::generate(
        &Preferences {
            theme_name: "First-Palette".into(),
            ..Default::default()
        },
    ))));
    for mode in ["light", "dark"] {
        let first_name = first["themes"]
            .as_array()
            .and_then(|themes| themes.iter().find(|theme| theme["mode"] == mode))
            .and_then(|theme| theme["name"].as_str())
            .unwrap_or_else(|| panic!("first theme missing: {mode}"));
        let second_name = second["themes"]
            .as_array()
            .and_then(|themes| themes.iter().find(|theme| theme["mode"] == mode))
            .and_then(|theme| theme["name"].as_str())
            .unwrap_or_else(|| panic!("second theme missing: {mode}"));
        assert_ne!(first_name, second_name);
        assert!(first_name.contains("First-Palette"));
        assert!(second_name.contains("Second-Palette"));
        let refreshed_name = refreshed["themes"]
            .as_array()
            .and_then(|themes| themes.iter().find(|theme| theme["mode"] == mode))
            .and_then(|theme| theme["name"].as_str())
            .unwrap_or_else(|| panic!("refreshed theme missing: {mode}"));
        assert_ne!(first_name, refreshed_name);
    }
}

#[test]
fn eight_semantic_families_cover_the_related_native_roles() {
    let mut p = Preferences::default();
    p.seed = "#22aaee".into();
    for (role, color) in [
        ("background", "#ff00cc"),
        ("foreground", "#112233"),
        ("secondary.background", "#668899"),
        ("success.background", "#11bb22"),
        ("warning.background", "#ddaa00"),
        ("danger.background", "#dd1122"),
        ("info.background", "#0088aa"),
    ] {
        p.manual_colors.insert(role.into(), color.into());
    }
    let value: serde_json::Value = must(serde_json::from_str(&must(generate::generate(&p))));
    let themes = value["themes"]
        .as_array()
        .unwrap_or_else(|| panic!("themes missing"));
    for theme in themes {
        let colors = &theme["colors"];
        for role in [
            "group_box.background",
            "popover.background",
            "muted.background",
            "switch.background",
        ] {
            assert_eq!(colors[role], "#ff00cc", "{role}");
        }
        for role in [
            "button.foreground",
            "popover.foreground",
            "group_box.foreground",
        ] {
            assert_eq!(colors[role], "#112233", "{role}");
        }
        for role in [
            "list.hover.background",
            "table.hover.background",
            "selection.background",
            "button.active.background",
            "button.secondary.active.background",
            "secondary.active.background",
            "list.active.background",
            "tab.active.background",
        ] {
            assert_eq!(colors[role], colors["accent.background"], "{role}");
        }
        for role in [
            "secondary.background",
            "button.secondary.background",
            "chart.4",
        ] {
            assert_eq!(colors[role], "#668899", "{role}");
        }
        for role in [
            "accent.background",
            "button.hover.background",
            "button.secondary.hover.background",
            "list.hover.background",
            "table.hover.background",
        ] {
            assert_eq!(colors[role], colors["accent.background"], "{role}");
        }
        for role in [
            "button.success.background",
            "success.hover.background",
            "chart.2",
            "base.green",
        ] {
            assert_eq!(colors[role], "#11bb22", "{role}");
        }
        for role in [
            "button.warning.background",
            "warning.active.background",
            "chart.3",
            "base.yellow",
        ] {
            assert_eq!(colors[role], "#ddaa00", "{role}");
        }
        for role in [
            "button.danger.background",
            "danger.hover.background",
            "chart.bearish",
            "base.red",
        ] {
            assert_eq!(colors[role], "#dd1122", "{role}");
        }
        for role in [
            "button.info.background",
            "info.hover.background",
            "chart.5",
            "base.cyan",
            "base.magenta",
        ] {
            assert_eq!(colors[role], "#0088aa", "{role}");
        }
        for (background, foreground) in [
            ("secondary.background", "secondary.foreground"),
            ("success.background", "success.foreground"),
            ("warning.background", "warning.foreground"),
            ("danger.background", "danger.foreground"),
            ("info.background", "info.foreground"),
        ] {
            let background =
                rgb_value(parse_hex_color(colors[background].as_str().unwrap()).unwrap());
            let foreground =
                rgb_value(parse_hex_color(colors[foreground].as_str().unwrap()).unwrap());
            assert!(
                contrast_ratio(foreground, background) >= 4.5,
                "{foreground:#08x} on {background:#08x}"
            );
        }
    }
}

#[test]
fn named_generation_preserves_default_and_uses_safe_output_path() {
    let path =
        std::env::temp_dir().join(format!("fileatlas-material-named-{}", std::process::id()));
    let service = MaterialThemeService::open(Arc::new(SettingsService::open_at(
        path.join("settings.json"),
    )));
    let default = must(service.generate(Preferences::default(), false));
    let named = must(service.generate_named(Preferences::default(), "My-Theme_2".into(), false));
    assert!(named.output.ends_with("My-Theme_2.json"));
    assert!(default.output.exists());
    assert!(named.output.exists());
    let original = must(std::fs::read(&named.output));
    let mut replacement = Preferences::default();
    replacement.seed = "#cc2244".into();
    let updated = must(service.generate_named(replacement, "My-Theme_2".into(), false));
    assert_eq!(updated.output, named.output);
    assert_ne!(must(std::fs::read(&named.output)), original);
    assert_eq!(
        must(std::fs::read(named.output.with_extension("json.bak"))),
        original
    );

    let overwritten = must(std::fs::read(&named.output));
    let mut invalid = Preferences::default();
    invalid
        .manual_colors
        .insert("not.a.role".into(), "#fff".into());
    assert!(
        service
            .generate_named(invalid, "My-Theme_2".into(), false)
            .is_err()
    );
    assert_eq!(must(std::fs::read(&named.output)), overwritten);
    let reopened = MaterialThemeService::open(Arc::new(SettingsService::open_at(
        path.join("settings.json"),
    )));
    assert_eq!(reopened.snapshot().output, named.output);
    assert!(reopened.snapshot().generated);
    assert!(
        service
            .generate_named(Preferences::default(), "../../escape".into(), false)
            .is_err()
    );
    must(std::fs::remove_dir_all(path));
}

#[test]
fn version_one_preferences_migrate_without_losing_generator_inputs() {
    let path =
        std::env::temp_dir().join(format!("fileatlas-material-migrate-{}", std::process::id()));
    let root = path.join("themes");
    must(std::fs::create_dir_all(&root));
    must(std::fs::write(
        root.join("material-settings.json"),
        r##"{"schema_version":1,"seed":"#123456","variant":"monochrome","contrast":0.5}"##,
    ));
    let service = MaterialThemeService::open(Arc::new(SettingsService::open_at(
        path.join("settings.json"),
    )));
    let prefs = service.snapshot().preferences;
    assert_eq!(prefs.schema_version, 3);
    assert_eq!(prefs.seed, "#123456");
    assert_eq!(prefs.variant, "monochrome");
    assert!(prefs.manual_colors.is_empty());
    assert_eq!(prefs.theme_name, "material");
    must(std::fs::remove_dir_all(path));
}

#[test]
fn version_two_advanced_colors_fold_into_eight_semantic_families() {
    let legacy = Preferences {
        schema_version: 2,
        seed: "#123456".into(),
        variant: "monochrome".into(),
        contrast: 0.5,
        manual_colors: [
            ("accent.background".into(), "#aa0000".into()),
            ("popover.background".into(), "#ffeecc".into()),
            ("button.warning.hover.background".into(), "#cc8800".into()),
            ("button.info.background".into(), "#0088aa".into()),
            ("button.secondary.background".into(), "#668899".into()),
        ]
        .into_iter()
        .collect(),
        theme_name: "legacy".into(),
    };
    let migrated = migrate_preferences(legacy).unwrap();
    assert_eq!(migrated.schema_version, 3);
    assert_eq!(migrated.seed, "#aa0000");
    assert_eq!(migrated.manual_colors["background"], "#ffeecc");
    assert_eq!(migrated.manual_colors["warning.background"], "#cc8800");
    assert_eq!(migrated.manual_colors["info.background"], "#0088aa");
    assert_eq!(migrated.manual_colors["secondary.background"], "#668899");
    assert!(
        migrated
            .manual_colors
            .keys()
            .all(|role| PALETTE_ROLES.contains(&role.as_str()))
    );
}
#[test]
fn material_json_resolves_native_roles_with_contrast_for_extreme_seeds() {
    for seed in ["#000000", "#ffffff", "#6686a3"] {
        for variant in ["neutral", "tonal-spot", "monochrome"] {
            for contrast in [0., 1.] {
                let p = Preferences {
                    seed: seed.into(),
                    variant: variant.into(),
                    contrast,
                    ..Default::default()
                };
                let json = generate::generate(&p).unwrap();
                let set: ThemeSet = serde_json::from_str(&json).unwrap();
                assert_eq!(set.themes.len(), 2);
                for config in set.themes {
                    let mut native = Theme::default();
                    native.apply_config(&std::rc::Rc::new(config.clone()));
                    let colors = native.colors;
                    for (foreground, background) in [
                        (colors.foreground, colors.background),
                        (colors.primary_foreground, colors.primary),
                        (colors.danger_foreground, colors.danger),
                        (colors.warning_foreground, colors.warning),
                        (colors.success_foreground, colors.success),
                        (colors.accent_foreground, colors.accent),
                        (colors.muted_foreground, colors.muted),
                    ] {
                        assert!(
                            contrast_ratio(packed(foreground), packed(background)) >= 4.5,
                            "{seed} {variant} {contrast} {:?}",
                            config.mode
                        );
                    }
                    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
                    let expected = value["themes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|t| t["name"].as_str() == Some(config.name.as_ref()))
                        .unwrap();
                    let recognized = serde_json::to_value(&config.colors).unwrap();
                    for key in expected["colors"].as_object().unwrap().keys() {
                        assert!(
                            recognized.get(key).is_some(),
                            "unknown native ThemeConfig role: {key}"
                        );
                    }
                    assert_eq!(
                        format!("#{:06x}", packed(colors.background)),
                        expected["colors"]["background"].as_str().unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn every_native_color_role_has_a_generated_light_and_dark_preview_value() {
    let valid: serde_json::Value = must(serde_json::to_value(
        gpui_kit::component::ThemeConfigColors::default(),
    ));
    let json = must(generate::generate(&Preferences::default()));
    let generated: serde_json::Value = must(serde_json::from_str(&json));
    let roles = valid
        .as_object()
        .unwrap_or_else(|| panic!("native ThemeConfigColors must serialize as an object"));
    let mut missing = Vec::new();
    for config in generated["themes"]
        .as_array()
        .unwrap_or_else(|| panic!("generated ThemeSet must contain themes"))
    {
        let colors = config["colors"]
            .as_object()
            .unwrap_or_else(|| panic!("generated theme colors must be an object"));
        for role in roles.keys() {
            if colors
                .get(role)
                .and_then(serde_json::Value::as_str)
                .is_none()
            {
                missing.push(format!("{}:{role}", config["mode"]));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "missing native role values: {missing:?}"
    );
}

#[test]
fn invalid_generation_preserves_previous_json_and_recovery_files() {
    let path = std::env::temp_dir().join(format!(
        "fileatlas-material-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let service = MaterialThemeService::open(Arc::new(SettingsService::open_at(
        path.join("settings.json"),
    )));
    let state = service.generate(Preferences::default(), false).unwrap();
    let first = std::fs::read(&state.output).unwrap();
    let mut bad = Preferences::default();
    bad.seed = "你好".into();
    assert!(service.generate(bad, false).is_err());
    assert_eq!(std::fs::read(&state.output).unwrap(), first);
    let mut next = Preferences::default();
    next.variant = "monochrome".into();
    let replaced = service.generate(next.clone(), false).unwrap();
    assert_ne!(must(std::fs::read(&state.output)), first);
    assert_eq!(
        must(std::fs::read(state.output.with_extension("json.bak"))),
        first
    );
    assert_eq!(replaced.output, state.output);
    let named = service
        .generate_named(next, "monochrome".into(), false)
        .unwrap();
    assert!(named.output.exists());
    let prefs = path.join("themes/material-settings.json");
    std::fs::write(&prefs, "{broken}").unwrap();
    let reopened = MaterialThemeService::open(Arc::new(SettingsService::open_at(
        path.join("settings.json"),
    )));
    assert!(reopened.snapshot().error.is_some());
    assert_eq!(std::fs::read_to_string(prefs).unwrap(), "{broken}");
    std::fs::remove_dir_all(path).unwrap();
}
