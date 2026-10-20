use super::*;
use fileatlas_ui_bridge::theme::{contrast_ratio, packed};
use gpui_kit::component::{Theme, ThemeSet};
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
    service.generate(next, false).unwrap();
    assert_eq!(
        std::fs::read(state.output.with_extension("json.bak")).unwrap(),
        first
    );
    let prefs = path.join("themes/material-settings.json");
    std::fs::write(&prefs, "{broken}").unwrap();
    let reopened = MaterialThemeService::open(Arc::new(SettingsService::open_at(
        path.join("settings.json"),
    )));
    assert!(reopened.snapshot().error.is_some());
    assert_eq!(std::fs::read_to_string(prefs).unwrap(), "{broken}");
    std::fs::remove_dir_all(path).unwrap();
}
