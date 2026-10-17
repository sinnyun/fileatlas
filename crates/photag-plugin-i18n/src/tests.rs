use super::*;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "photag-i18n-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn both_catalogs_format_every_message_and_preserve_arguments() {
    let service = I18nService::open(temporary("catalog/language.json")).unwrap();
    let args: [(&str, String); 19] = [
        ("count", "9".into()),
        ("folders", "2".into()),
        ("files", "5".into()),
        ("query", "sample.jpg".into()),
        ("key", "Space".into()),
        ("photos", "8".into()),
        ("videos", "0".into()),
        ("total", "8".into()),
        ("size", "0.01".into()),
        ("month", "October".into()),
        ("year", "2026".into()),
        ("action", "Delete".into()),
        ("error", "permission denied".into()),
        ("records", "9".into()),
        ("path", "backup.json".into()),
        ("modifier", "Ctrl".into()),
        ("scanned", "120".into()),
        ("base", "01 original".into()),
        ("matched", "02 edited".into()),
    ];
    for key in service.keys.values() {
        let mut fluent_args = FluentArgs::new();
        for (name, value) in &args {
            fluent_args.set(*name, value.as_str());
        }
        assert!(
            service
                .format(&service.english, key, &fluent_args)
                .is_some(),
            "English {key}"
        );
        assert!(
            service
                .format(&service.chinese, key, &fluent_args)
                .is_some(),
            "Chinese {key}"
        );
    }
    assert_eq!(service.translate("{count} selected", &args), "9 selected");
    assert_eq!(
        service.translate("Matches: {count} items", &[("count", "1".into())]),
        "Matches: 1 item"
    );
    *service.language.write().unwrap() = Language::Chinese;
    assert_eq!(service.translate("{count} selected", &args), "已选择 9 项");
    assert_eq!(service.translate("✓ Photos", &[]), "✓ 照片");
    assert_eq!(
        service.translate("{folders} folders · {files} files", &args),
        "2 个文件夹 · 5 个文件"
    );
    assert_eq!(
        service.translate("user-created tag", &[]),
        "user-created tag"
    );
}

#[test]
fn default_is_english_and_language_persists_without_changing_other_settings() {
    let path = temporary("persist/language.json");
    let service = I18nService::open(path.clone()).unwrap();
    assert_eq!(service.language(), Language::English);
    service.set_language(Language::Chinese).unwrap();
    assert_eq!(
        I18nService::open(path.clone()).unwrap().language(),
        Language::Chinese
    );
    assert!(Language::parse("fr").is_err());
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn save_failure_does_not_switch_visible_language() {
    let path = temporary("blocked");
    std::fs::write(&path, b"parent is a file").unwrap();
    let service = I18nService::open(path.join("language.json")).unwrap();
    assert!(service.set_language(Language::Chinese).is_err());
    assert_eq!(service.language(), Language::English);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn cordis_stop_removes_translation_and_language_view() {
    use cordis::{Context, PreparedPlugin};
    use photag_ui_bridge::{UiBridgeService, UiCatalog};
    let context = Context::new();
    let bridge = Arc::new(UiBridgeService::new(Arc::new(UiCatalog::default())));
    let _publication = context.provide(bridge.clone()).unwrap();
    let path = temporary("lifecycle/language.json");
    let plugin = context
        .spawn(PreparedPlugin::from_input(
            I18nPlugin,
            I18nConfig {
                path: Some(path.clone()),
            },
        ))
        .await
        .unwrap();
    let service = context.try_service::<I18nService>().unwrap();
    service.set_language(Language::Chinese).unwrap();
    assert_eq!(photag_ui_bridge::i18n::tr("Settings"), "设置");
    assert!(
        bridge
            .views
            .snapshot()
            .unwrap()
            .iter()
            .any(|view| view.owner == "photag.i18n")
    );
    plugin.dispose().await.unwrap();
    assert_eq!(photag_ui_bridge::i18n::tr("Settings"), "Settings");
    assert!(bridge.views.snapshot().unwrap().is_empty());
    assert!(context.try_service::<I18nService>().is_err());
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}
