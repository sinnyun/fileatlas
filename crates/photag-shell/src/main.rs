//! Native GPUI host and Cordis runtime composition.

use gpui_kit::prelude::FluentBuilder;

use std::sync::Arc;

fn dev_plugin_enabled(config: &str, name: &str) -> bool {
    config == "all" || config.split(',').any(|plugin| plugin.trim() == name)
}

fn dev_plugin_config() -> String {
    let config = std::env::var("PHOTAG_DEV_PLUGINS").unwrap_or_else(|_| "core".to_owned());
    let valid = [
        "core",
        "all",
        "gallery",
        "navigation",
        "metadata-export",
        "file-operations",
        "thumbnails",
        "indexing",
        "watcher",
        "tagging",
        "history",
        "media-playback",
        "drag-drop",
        "mcp",
        "dev-test",
    ];
    for plugin in config
        .split(',')
        .map(str::trim)
        .filter(|plugin| !plugin.is_empty())
    {
        if !valid.contains(&plugin) {
            tracing::error!("Unknown PHOTAG_DEV_PLUGINS entry: {plugin}");
            eprintln!("Unknown PHOTAG_DEV_PLUGINS entry: {plugin}");
            eprintln!("Valid entries: {}", valid.join(", "));
            std::process::exit(2);
        }
    }
    config
}

use cordis::{Context as CordisContext, Plugin, PreparedPlugin, Routing};
use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::*;
use gpui_kit::*;
use photag_ui_bridge::{
    ContributionKind, OwnedContribution, UiBridgeService, UiCatalog, UiCommandInvoked, theme,
};

mod shell;
use shell::PhoTagShell;

fn render_contribution(owned: &OwnedContribution) -> Div {
    let contribution = &owned.contribution;
    let (category, location) = match &contribution.kind {
        ContributionKind::Layout { label, .. } => ("Layout", label.clone()),
        ContributionKind::Dock { label, .. } => ("Dock", label.clone()),
        ContributionKind::Panel { slot, .. } => ("Panel", slot.clone()),
        ContributionKind::Slot { label, .. } => ("Slot", label.clone()),
        ContributionKind::Widget { slot, .. } => ("Widget", slot.clone()),
        ContributionKind::Command { label } => ("Command", label.clone()),
        ContributionKind::MenuItem { menu, .. } => ("Menu", menu.clone()),
        ContributionKind::Keybinding { key, .. } => ("Keybinding", key.clone()),
    };
    let title = match &contribution.kind {
        ContributionKind::Panel { label, .. }
        | ContributionKind::Widget { label, .. }
        | ContributionKind::MenuItem { label, .. } => label.clone(),
        _ => contribution.id.clone(),
    };
    div()
        .h_flex()
        .w_full()
        .items_center()
        .gap_3()
        .rounded_md()
        .border_1()
        .border_color(rgba(theme::BORDER.to_rgba()))
        .bg(rgb(theme::SURFACE_PANEL))
        .px_3()
        .py_2()
        .child(
            div()
                .w_24()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(category),
        )
        .child(div().flex_1().child(title))
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_SECONDARY))
                .child(location),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::TEXT_MUTED))
                .child(owned.owner.clone()),
        )
}

fn main() {
    let diagnostics = Arc::new(photag_plugin_debug::DebugService::default());
    photag_plugin_debug::attach(&diagnostics).expect("install runtime diagnostics");
    let plugin_config = dev_plugin_config();
    let mcp_arg = std::env::args().skip(1).any(|arg| arg == "--mcp");
    let enable_mcp = mcp_arg
        || plugin_config
            .split(',')
            .any(|plugin| plugin.trim() == "mcp");
    let enable_all = dev_plugin_enabled(&plugin_config, "all");
    let enable_dev_test = dev_plugin_enabled(&plugin_config, "dev-test");
    let enable_history = enable_all || dev_plugin_enabled(&plugin_config, "history");
    let enable_gallery = enable_all
        || enable_mcp
        || enable_history
        || enable_dev_test
        || dev_plugin_enabled(&plugin_config, "gallery");
    let enable_ai =
        enable_all || enable_mcp || enable_gallery || dev_plugin_enabled(&plugin_config, "ai");
    let enable_navigation = enable_all
        || enable_gallery
        || enable_mcp
        || enable_dev_test
        || dev_plugin_enabled(&plugin_config, "navigation");
    let enable_metadata_export =
        enable_all || enable_mcp || dev_plugin_enabled(&plugin_config, "metadata-export");
    let enable_file_operations = enable_all
        || enable_mcp
        || enable_gallery
        || dev_plugin_enabled(&plugin_config, "file-operations");
    let enable_thumbnails = enable_all
        || enable_mcp
        || enable_navigation
        || enable_dev_test
        || dev_plugin_enabled(&plugin_config, "thumbnails")
        || dev_plugin_enabled(&plugin_config, "watcher");
    let enable_indexing = enable_all
        || enable_mcp
        || enable_navigation
        || enable_dev_test
        || dev_plugin_enabled(&plugin_config, "indexing")
        || dev_plugin_enabled(&plugin_config, "watcher");
    let enable_watcher = enable_all
        || enable_mcp
        || enable_navigation
        || dev_plugin_enabled(&plugin_config, "watcher");
    let enable_tagging = enable_all
        || enable_mcp
        || enable_gallery
        || enable_history
        || dev_plugin_enabled(&plugin_config, "tagging");
    let enable_history = enable_history || (enable_gallery && enable_tagging);
    let enable_media_playback = enable_all
        || enable_mcp
        || enable_gallery
        || dev_plugin_enabled(&plugin_config, "media-playback");
    let enable_drag_drop = enable_all
        || enable_mcp
        || enable_gallery
        || dev_plugin_enabled(&plugin_config, "drag-drop");
    tracing::info!("Starting development plugins: {plugin_config}");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create Cordis async runtime");
    let cordis = CordisContext::new();
    let capture_handle = Arc::new(photag_plugin_mcp::CaptureWindowHandle::default());
    let _capture_publication = cordis
        .provide(Arc::clone(&capture_handle))
        .expect("publish MCP window capture handle");
    let catalog = Arc::new(UiCatalog::default());
    let bridge = Arc::new(UiBridgeService::new(Arc::clone(&catalog)));
    let _bridge_publication = cordis
        .provide(Arc::clone(&bridge))
        .expect("publish UI bridge service");

    let debug_plugin = photag_plugin_debug::DebugPlugin;
    let debug_input = debug_plugin
        .prepare(diagnostics)
        .expect("prepare debug plugin");
    let debug_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(debug_plugin, debug_input)))
        .expect("start debug plugin");

    let dev_test_dir = enable_dev_test.then(|| {
        std::env::var_os("PHOTAG_DEV_TEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dev-test")
            })
    });
    let i18n_plugin = photag_plugin_i18n::I18nPlugin;
    let i18n_input = i18n_plugin
        .prepare(photag_plugin_i18n::I18nConfig {
            path: dev_test_dir.as_ref().map(|path| path.join("language.json")),
        })
        .expect("prepare i18n plugin");
    let i18n_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(i18n_plugin, i18n_input)))
        .expect("start i18n plugin");
    let session_plugin = photag_plugin_session::SessionPlugin;
    let session_config = photag_plugin_session::SessionConfig {
        path: dev_test_dir.as_ref().map(|path| path.join("session.json")),
    };
    let session_input = session_plugin
        .prepare(session_config)
        .expect("prepare session plugin");
    let session_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(session_plugin, session_input)))
        .expect("start session plugin");
    let updater_plugin = photag_plugin_updater::UpdaterPlugin;
    let updater_input = updater_plugin
        .prepare(photag_plugin_updater::UpdaterConfig)
        .expect("prepare updater plugin");
    let updater_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(updater_plugin, updater_input)))
        .expect("start updater plugin");
    let database_plugin = photag_plugin_database::DatabasePlugin;
    let database_path = dev_test_dir.as_ref().map(|path| {
        std::env::var_os("PHOTAG_DEV_TEST_DB")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| path.join("photag.db"))
    });
    let database_input = database_plugin
        .prepare(photag_plugin_database::DatabaseConfig {
            path: database_path,
        })
        .expect("prepare database plugin");
    let database_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(database_plugin, database_input)))
        .expect("start database plugin");
    let file_operations_fiber = if enable_file_operations {
        let plugin = photag_plugin_file_operations::FileOperationsPlugin;
        let input = plugin
            .prepare(photag_plugin_file_operations::FileOperationsConfig)
            .expect("prepare file operations plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start file operations plugin"),
        )
    } else {
        None
    };
    let drag_drop_fiber = if enable_drag_drop {
        let plugin = photag_plugin_drag_drop::DragDropPlugin;
        let input = plugin
            .prepare(photag_plugin_drag_drop::DragDropConfig)
            .expect("prepare drag-drop plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start drag-drop plugin"),
        )
    } else {
        None
    };
    let metadata_export_fiber = if enable_metadata_export {
        let plugin = photag_plugin_metadata_export::MetadataExportPlugin;
        let input = plugin
            .prepare(photag_plugin_metadata_export::MetadataExportConfig)
            .expect("prepare metadata export plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start metadata export plugin"),
        )
    } else {
        None
    };
    let thumbnail_fiber = if enable_thumbnails {
        let plugin = photag_plugin_thumbnails::ThumbnailPlugin;
        let thumbnail_config = photag_plugin_thumbnails::ThumbnailConfig {
            directory: dev_test_dir.as_ref().map(|path| path.join("thumbs")),
        };
        let input = plugin
            .prepare(thumbnail_config)
            .expect("prepare thumbnail plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start thumbnail plugin"),
        )
    } else {
        None
    };
    let indexing_fiber = if enable_indexing {
        let plugin = photag_plugin_indexing::IndexingPlugin;
        let input = plugin
            .prepare(photag_plugin_indexing::IndexingConfig)
            .expect("prepare indexing plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start indexing plugin"),
        )
    } else {
        None
    };
    let watcher_fiber = if enable_watcher {
        let plugin = photag_plugin_watcher::WatcherPlugin;
        let input = plugin
            .prepare(photag_plugin_watcher::WatcherConfig)
            .expect("prepare watcher plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start watcher plugin"),
        )
    } else {
        None
    };
    let navigation_fiber = if enable_navigation {
        let plugin = photag_plugin_navigation::NavigationPlugin;
        let input = plugin
            .prepare(photag_plugin_navigation::NavigationConfig)
            .expect("prepare navigation plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start navigation plugin"),
        )
    } else {
        None
    };
    // Settings can read the live tag tree from NavigationService when the
    // navigation plugin is enabled, so start it after Navigation.
    let settings_plugin = photag_plugin_settings::SettingsPlugin;
    let settings_config = photag_plugin_settings::SettingsConfig {
        path: dev_test_dir.as_ref().map(|path| path.join("settings.json")),
    };
    let settings_input = settings_plugin
        .prepare(settings_config)
        .expect("prepare settings plugin");
    let settings_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(settings_plugin, settings_input)))
        .expect("start settings plugin");
    // Start AI after Settings so future consumers can read the user's
    // mandatory tag groups before requesting suggestions.
    let ai_fiber = if enable_ai {
        let plugin = photag_plugin_ai::AiPlugin;
        let input = plugin
            .prepare(photag_plugin_ai::AiConfig)
            .expect("prepare AI plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start AI plugin"),
        )
    } else {
        None
    };
    let tagging_fiber = if enable_tagging {
        let plugin = photag_plugin_tagging::TaggingPlugin;
        let input = plugin
            .prepare(photag_plugin_tagging::TaggingConfig)
            .expect("prepare tagging plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start tagging plugin"),
        )
    } else {
        None
    };
    let history_fiber = if enable_history {
        let plugin = photag_plugin_history::HistoryPlugin;
        let input = plugin
            .prepare(photag_plugin_history::HistoryConfig)
            .expect("prepare history plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start history plugin"),
        )
    } else {
        None
    };
    // Preview resolves AI, Settings, Tagging, and History while applying its
    // UI integration, so it starts after all four providers.
    let preview_fiber = if enable_gallery {
        let plugin = photag_plugin_preview::PreviewPlugin;
        let input = plugin
            .prepare(photag_plugin_preview::PreviewConfig::default())
            .expect("prepare image preview plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start image preview plugin"),
        )
    } else {
        None
    };
    let media_playback_fiber = if enable_media_playback {
        let plugin = photag_plugin_media_playback::MediaPlaybackPlugin;
        let input = plugin
            .prepare(photag_plugin_media_playback::MediaPlaybackConfig::default())
            .expect("prepare media playback plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start media playback plugin"),
        )
    } else {
        None
    };
    // Gallery requires the media-playback service and optionally consumes
    // tagging/history services, so start it after all of those providers.
    let grouping_fiber = if enable_gallery {
        let plugin = photag_plugin_grouping::GroupingPlugin;
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ())))
                .expect("start grouping plugin"),
        )
    } else {
        None
    };
    let gallery_fiber = if enable_gallery {
        let plugin = photag_plugin_gallery::GalleryPlugin;
        let input = plugin
            .prepare(photag_plugin_gallery::GalleryConfig)
            .expect("prepare gallery plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start gallery plugin"),
        )
    } else {
        None
    };
    let dev_test_fiber = if enable_dev_test {
        let plugin = photag_plugin_dev_test::DevTestPlugin;
        let input = plugin
            .prepare(photag_plugin_dev_test::DevTestConfig)
            .expect("prepare development test plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start development test plugin"),
        )
    } else {
        None
    };
    let mcp_fiber = if enable_mcp {
        let plugin = photag_plugin_mcp::McpPlugin;
        let input = plugin.prepare(()).expect("prepare MCP plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start MCP plugin"),
        )
    } else {
        None
    };
    let executor = runtime.handle().clone();
    let app_cordis = cordis.clone();

    let application = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets);
    application.run(move |cx| {
        gpui_kit::init(cx);
        if let Ok(i18n) = app_cordis.try_service::<photag_plugin_i18n::I18nService>() {
            gpui_kit::component::set_locale(i18n.language().code());
        }
        photag_ui_bridge::theme::apply_components(cx);
        let catalog = Arc::clone(&catalog);
        let views = Arc::clone(&bridge.views);
        let cordis = app_cordis.clone();
        let executor = executor.clone();
        let capture_handle = Arc::clone(&capture_handle);
        let session = cordis
            .try_service::<photag_plugin_session::SessionService>()
            .expect("session service started before shell");
        let sidebar_width = session
            .snapshot()
            .ok()
            .map(|saved| saved.sidebar_width)
            .filter(|width| {
                width.is_finite()
                    && (theme::SIDEBAR_MIN_WIDTH..=theme::SIDEBAR_MAX_WIDTH).contains(width)
            })
            // Source: vendor/photag-source/src/app.rs::AppState::new.
            .unwrap_or(theme::SIDEBAR_WIDTH);
        let inspector_width = session
            .snapshot()
            .ok()
            .map(|saved| saved.inspector_width)
            .filter(|width| width.is_finite() && (240. ..=480.).contains(width))
            .unwrap_or(theme::INSPECTOR_WIDTH);
        let session_for_shell = Arc::clone(&session);
        // Match the source viewport (1280×800, minimum 800×500) from
        // vendor/photag-source/src/main.rs.
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(
                size(px(theme::WINDOW_WIDTH), px(theme::WINDOW_HEIGHT)),
                cx,
            )),
            window_min_size: Some(size(
                px(theme::WINDOW_MIN_WIDTH),
                px(theme::WINDOW_MIN_HEIGHT),
            )),
            ..WindowOptions::default()
        };
        gpui_kit::open_window(window_options, cx, move |window, cx| {
            window.set_window_title("PhoTag");
            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let hwnd = window
                    .window_handle()
                    .ok()
                    .and_then(|handle| match handle.as_raw() {
                        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as isize),
                        _ => None,
                    });
                capture_handle.set(hwnd.unwrap_or_default());
                let backdrop = hwnd
                    .map(photag_ui_bridge::winfx::apply_backdrop)
                    .unwrap_or(photag_ui_bridge::winfx::Backdrop::None);
                photag_ui_bridge::theme::set_backdrop_active(backdrop.is_translucent());
            }
            #[cfg(not(target_os = "windows"))]
            {
                capture_handle.set(1);
                photag_ui_bridge::theme::set_backdrop_active(false);
            }
            let mounted_views = views
                .snapshot()
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| entry.contribution.slot != "settings.language")
                .map(|entry| {
                    let slot = entry.contribution.slot.clone();
                    let view = (entry.contribution.factory)(window, cx);
                    (
                        entry.contribution.id,
                        entry.generation,
                        slot,
                        entry.contribution.panel_id,
                        view,
                    )
                })
                .collect();
            if let Err(error) = bridge.surfaces.set_open("inspector", true) {
                tracing::error!("inspector initialization failed: {error}");
            }
            cx.new(|cx| {
                let mut locale =
                    photag_ui_bridge::refresh::ChangeGate::new(photag_ui_bridge::i18n::revision());
                photag_ui_bridge::refresh::poll(
                    cx,
                    std::time::Duration::from_millis(100),
                    move |shell: &mut PhoTagShell, cx| {
                        let mut changed = locale.changed(photag_ui_bridge::i18n::revision());
                        if changed {
                            if let Ok(i18n) = shell
                                .cordis
                                .try_service::<photag_plugin_i18n::I18nService>()
                            {
                                gpui_kit::component::set_locale(i18n.language().code());
                            }
                            cx.refresh_windows();
                        }
                        shell.poll_panel_requests(cx);
                        let registered = shell
                            .view_catalog
                            .snapshot()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|entry| entry.contribution.slot != "settings.language")
                            .map(|entry| (entry.contribution.id, entry.generation))
                            .collect::<Vec<_>>();
                        if registered
                            != shell
                                .views
                                .iter()
                                .map(|(id, generation, _, _, _)| (id.clone(), *generation))
                                .collect::<Vec<_>>()
                        {
                            changed = true;
                        }

                        let panels = (
                            shell.surfaces.any_panel_open(),
                            shell.surfaces.value("panel.active"),
                        );
                        if panels != shell.panel_signature {
                            shell.panel_signature = panels;
                            changed = true;
                        }
                        let tools = shell.surfaces.is_open("tools");
                        let sidebar =
                            shell.surfaces.value("sidebar.visible").as_deref() != Some("false");
                        if sidebar != shell.sidebar_open {
                            shell.sidebar_open = sidebar;
                            changed = true;
                        }
                        if tools != shell.tools_open {
                            shell.tools_open = tools;
                            changed = true;
                        }
                        let next = shell.surfaces.is_open("inspector");
                        if next != shell.inspector_open {
                            shell.inspector_open = next;
                            changed = true;
                        }
                        changed.into()
                    },
                );
                PhoTagShell {
                    catalog,
                    views: mounted_views,
                    view_catalog: views.clone(),
                    cordis,
                    executor,
                    session: session_for_shell,
                    surfaces: Arc::clone(&bridge.surfaces),
                    sidebar_width,
                    inspector_width,
                    viewport_width: 1280.,
                    capture: Arc::clone(&capture_handle),
                    drag_origin_x: 0.,
                    drag_origin_width: 0.,
                    resizing_panel: None,
                    tools_open: false,
                    sidebar_open: true,
                    inspector_open: true,
                    panel_signature: (false, None),
                }
            })
        })
        .expect("open PhoTag window");
    });

    if let Ok(updater) = cordis.try_service::<photag_plugin_updater::UpdaterService>() {
        match updater.take_pending_install() {
            Ok(Some(staged)) => {
                if let Err(error) = updater.install(&staged) {
                    let log = dirs::data_local_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("PhoTag")
                        .join("update-error.txt");
                    let _ = std::fs::create_dir_all(
                        log.parent().unwrap_or_else(|| std::path::Path::new(".")),
                    );
                    let _ = std::fs::write(
                        &log,
                        format!(
                            "PhoTag {} could not install update from {}\n\n{}\n",
                            photag_plugin_updater::current_version(),
                            staged.display(),
                            error
                        ),
                    );
                    tracing::error!("update installation failed: {error}; see {}", log.display());
                }
            }
            Ok(None) => {}
            Err(error) => tracing::error!("could not read scheduled update: {error}"),
        }
    }

    if let Some(fiber) = mcp_fiber {
        runtime.block_on(fiber.dispose()).expect("stop MCP plugin");
    }
    if let Some(fiber) = dev_test_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop development test plugin");
    }
    if let Some(fiber) = gallery_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop gallery plugin");
    }
    if let Some(fiber) = grouping_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop grouping plugin");
    }
    if let Some(fiber) = preview_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop image preview plugin");
    }
    if let Some(fiber) = media_playback_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop media playback plugin");
    }
    if let Some(fiber) = history_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop history plugin");
    }
    if let Some(fiber) = ai_fiber {
        runtime.block_on(fiber.dispose()).expect("stop AI plugin");
    }
    runtime
        .block_on(settings_fiber.dispose())
        .expect("stop settings plugin");
    if let Some(fiber) = tagging_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop tagging plugin");
    }
    if let Some(fiber) = navigation_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop navigation plugin");
    }
    if let Some(fiber) = watcher_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop watcher plugin");
    }
    if let Some(fiber) = indexing_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop indexing plugin");
    }
    if let Some(fiber) = thumbnail_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop thumbnail plugin");
    }
    if let Some(fiber) = metadata_export_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop metadata export plugin");
    }
    if let Some(fiber) = file_operations_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop file operations plugin");
    }
    if let Some(fiber) = drag_drop_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop drag-drop plugin");
    }
    runtime
        .block_on(database_fiber.dispose())
        .expect("stop database plugin");
    runtime
        .block_on(updater_fiber.dispose())
        .expect("stop updater plugin");
    runtime
        .block_on(session_fiber.dispose())
        .expect("stop session plugin");
    runtime
        .block_on(i18n_fiber.dispose())
        .expect("stop i18n plugin");
    runtime
        .block_on(debug_fiber.dispose())
        .expect("stop debug plugin");
}
