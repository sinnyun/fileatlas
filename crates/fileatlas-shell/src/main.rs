//! Native GPUI host and Cordis runtime composition.

use gpui_kit::prelude::FluentBuilder;

use std::sync::Arc;

fn dev_plugin_enabled(config: &str, name: &str) -> bool {
    config == "all" || config.split(',').any(|plugin| plugin.trim() == name)
}

fn dev_plugin_config() -> String {
    let config = std::env::var("FILEATLAS_DEV_PLUGINS").unwrap_or_else(|_| "core".to_owned());
    let valid = [
        "core",
        "all",
        "file_manager",
        "columns",
        "workspaces",
        "navigation",
        "metadata-export",
        "launcher",
        "shortcuts",
        "clipboard",
        "file-operations",
        "thumbnails",
        "indexing",
        "watcher",
        "tagging",
        "history",
        "versions",
        "quicklook",
        "drag-drop",
        "mcp",
        "dev-test",
        "screenshot",
        "projects",
        "file-staging",
    ];
    for plugin in config
        .split(',')
        .map(str::trim)
        .filter(|plugin| !plugin.is_empty())
    {
        if !valid.contains(&plugin) {
            tracing::error!("Unknown FILEATLAS_DEV_PLUGINS entry: {plugin}");
            eprintln!("Unknown FILEATLAS_DEV_PLUGINS entry: {plugin}");
            eprintln!("Valid entries: {}", valid.join(", "));
            std::process::exit(2);
        }
    }
    config
}

use cordis::{Context as CordisContext, Plugin, PreparedPlugin, Routing};
use fileatlas_ui_bridge::{
    ContributionKind, OwnedContribution, UiBridgeService, UiCatalog, UiCommandInvoked, theme,
};
use gpui_kit::component::button::Button;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::*;
use gpui_kit::*;

mod shell;
use shell::FileAtlasShell;

/// Startup window extent: a share of the display work area, never below the
/// design baseline and never beyond the usable area itself.
fn startup_window_extent(available: Option<(f32, f32)>) -> (f32, f32) {
    let fit = |work_area: f32, baseline: f32| {
        let work_area = if work_area.is_finite() && work_area > 0.0 {
            work_area
        } else {
            baseline
        };
        (work_area * theme::WINDOW_WORK_AREA_FRACTION).clamp(baseline.min(work_area), work_area)
    };
    let (area_width, area_height) =
        available.unwrap_or((theme::WINDOW_WIDTH, theme::WINDOW_HEIGHT));
    (
        fit(area_width, theme::WINDOW_WIDTH),
        fit(area_height, theme::WINDOW_HEIGHT),
    )
}

fn render_contribution(owned: &OwnedContribution) -> Div {
    let contribution = &owned.contribution;
    let (category, location) = match &contribution.kind {
        ContributionKind::Layout { label, .. } => ("Layout", label.clone()),
        ContributionKind::Dock { label, .. } => ("Dock", label.clone()),
        ContributionKind::Panel { slot, .. } => ("Panel", slot.clone()),
        ContributionKind::Slot { label, .. } => ("Slot", label.clone()),
        ContributionKind::Widget { slot, .. } => ("Widget", slot.clone()),
        ContributionKind::Command { label, .. } => ("Command", label.clone()),
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
        .border_color(rgba(theme::border().to_rgba()))
        .bg(rgb(theme::surface_panel()))
        .px_3()
        .py_2()
        .child(
            div()
                .w_24()
                .text_xs()
                .text_color(rgb(theme::text_secondary()))
                .child(category),
        )
        .child(div().flex_1().child(title))
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::text_secondary()))
                .child(location),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::text_muted()))
                .child(owned.owner.clone()),
        )
}

fn main() {
    let diagnostics = Arc::new(fileatlas_plugin_debug::DebugService::default());
    fileatlas_plugin_debug::attach(&diagnostics).expect("install runtime diagnostics");
    let plugin_config = dev_plugin_config();
    let mcp_arg = std::env::args().skip(1).any(|arg| arg == "--mcp");
    // FileAtlas exposes its MCP endpoint from this GUI process on normal startup.
    // `--mcp` is retained only for isolated stdio-based development clients.
    let enable_mcp = true;
    let enable_all = dev_plugin_enabled(&plugin_config, "all");
    let enable_dev_test = dev_plugin_enabled(&plugin_config, "dev-test");
    let enable_history = enable_all || dev_plugin_enabled(&plugin_config, "history");
    let enable_file_manager = enable_all
        || enable_mcp
        || enable_history
        || enable_dev_test
        || dev_plugin_enabled(&plugin_config, "file_manager")
        || dev_plugin_enabled(&plugin_config, "versions");
    let enable_columns =
        enable_file_manager || enable_all || dev_plugin_enabled(&plugin_config, "columns");
    let enable_workspaces =
        enable_columns || enable_all || dev_plugin_enabled(&plugin_config, "workspaces");
    let enable_clipboard = plugin_config == "core"
        || enable_all
        || enable_mcp
        || dev_plugin_enabled(&plugin_config, "clipboard");
    let enable_screenshot = plugin_config == "core"
        || enable_all
        || enable_mcp
        || dev_plugin_enabled(&plugin_config, "screenshot");
    let enable_projects = plugin_config == "core"
        || enable_all
        || enable_mcp
        || dev_plugin_enabled(&plugin_config, "projects");
    let enable_file_staging = plugin_config == "core"
        || enable_all
        || enable_mcp
        || dev_plugin_enabled(&plugin_config, "file-staging");
    let enable_launcher = plugin_config == "core"
        || enable_all
        || enable_mcp
        || enable_file_manager
        || dev_plugin_enabled(&plugin_config, "launcher")
        || dev_plugin_enabled(&plugin_config, "shortcuts")
        || enable_clipboard
        || enable_screenshot;
    let enable_shortcuts = enable_launcher
        || enable_all
        || dev_plugin_enabled(&plugin_config, "shortcuts")
        || enable_clipboard
        || enable_screenshot;
    let enable_versions = enable_file_manager || dev_plugin_enabled(&plugin_config, "versions");
    let enable_ai =
        enable_all || enable_mcp || enable_file_manager || dev_plugin_enabled(&plugin_config, "ai");
    let enable_navigation = enable_all
        || enable_file_manager
        || enable_mcp
        || enable_dev_test
        || enable_launcher
        || dev_plugin_enabled(&plugin_config, "navigation")
        || enable_clipboard
        || enable_screenshot
        || enable_projects;
    let enable_metadata_export =
        enable_all || enable_mcp || dev_plugin_enabled(&plugin_config, "metadata-export");
    let enable_file_operations = enable_all
        || enable_mcp
        || enable_file_manager
        || dev_plugin_enabled(&plugin_config, "file-operations");
    let enable_thumbnails = enable_all
        || enable_mcp
        || enable_versions
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
        || enable_file_manager
        || enable_history
        || dev_plugin_enabled(&plugin_config, "tagging");
    let enable_history = enable_history || (enable_file_manager && enable_tagging);
    let enable_drag_drop = enable_all
        || enable_mcp
        || enable_file_manager
        || dev_plugin_enabled(&plugin_config, "drag-drop");
    tracing::info!("Starting development plugins: {plugin_config}");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create Cordis async runtime");
    let cordis = CordisContext::new();
    let capture_handle = Arc::new(fileatlas_plugin_mcp::CaptureWindowHandle::default());
    let _capture_publication = cordis
        .provide(Arc::clone(&capture_handle))
        .expect("publish MCP window capture handle");
    let mcp_tool_registry = Arc::new(fileatlas_contracts::mcp::McpToolRegistry::default());
    let _mcp_tool_registry_publication = match cordis.provide(Arc::clone(&mcp_tool_registry)) {
        Ok(publication) => publication,
        Err(error) => {
            tracing::error!("could not publish MCP tool contribution registry: {error}");
            eprintln!("Could not publish MCP tool contribution registry: {error}");
            std::process::exit(1);
        }
    };
    let catalog = Arc::new(UiCatalog::default());
    let bridge = Arc::new(UiBridgeService::new(Arc::clone(&catalog)));
    let _bridge_publication = cordis
        .provide(Arc::clone(&bridge))
        .expect("publish UI bridge service");

    let shell_controls_plugin = fileatlas_plugin_shell_controls::ShellControlsPlugin;
    let _shell_controls_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(shell_controls_plugin, ())))
        .expect("start shell controls plugin");

    let debug_plugin = fileatlas_plugin_debug::DebugPlugin;
    let debug_input = debug_plugin
        .prepare(diagnostics)
        .expect("prepare debug plugin");
    let debug_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(debug_plugin, debug_input)))
        .expect("start debug plugin");

    let dev_test_dir = enable_dev_test.then(|| {
        std::env::var_os("FILEATLAS_DEV_TEST_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dev-test")
            })
    });
    let i18n_plugin = fileatlas_plugin_i18n::I18nPlugin;
    let i18n_input = i18n_plugin
        .prepare(fileatlas_plugin_i18n::I18nConfig {
            path: dev_test_dir.as_ref().map(|path| path.join("language.json")),
        })
        .expect("prepare i18n plugin");
    let i18n_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(i18n_plugin, i18n_input)))
        .expect("start i18n plugin");
    let session_plugin = fileatlas_plugin_session::SessionPlugin;
    let session_config = fileatlas_plugin_session::SessionConfig {
        path: dev_test_dir.as_ref().map(|path| path.join("session.json")),
    };
    let session_input = session_plugin
        .prepare(session_config)
        .expect("prepare session plugin");
    let session_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(session_plugin, session_input)))
        .expect("start session plugin");
    let _workspace_fiber = if enable_workspaces {
        let plugin = fileatlas_plugin_workspaces::WorkspacesPlugin;
        let input = fileatlas_plugin_workspaces::WorkspacesConfig {
            path: dev_test_dir
                .as_ref()
                .map(|path| path.join("workspaces.json")),
        };
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start program workspaces plugin");
                None
            }
        }
    } else {
        None
    };
    let _columns_fiber = if enable_columns {
        let plugin = fileatlas_plugin_columns::ColumnsPlugin;
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ()))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start file_manager columns plugin");
                None
            }
        }
    } else {
        None
    };
    let updater_plugin = fileatlas_plugin_updater::UpdaterPlugin;
    let updater_input = updater_plugin
        .prepare(fileatlas_plugin_updater::UpdaterConfig)
        .expect("prepare updater plugin");
    let updater_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(updater_plugin, updater_input)))
        .expect("start updater plugin");
    let database_plugin = fileatlas_plugin_database::DatabasePlugin;
    let database_path = dev_test_dir.as_ref().map(|path| {
        std::env::var_os("FILEATLAS_DEV_TEST_DB")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| path.join("fileatlas.db"))
    });
    let database_input = database_plugin
        .prepare(fileatlas_plugin_database::DatabaseConfig {
            path: database_path,
        })
        .expect("prepare database plugin");
    let database_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(database_plugin, database_input)))
        .expect("start database plugin");
    let file_operations_fiber = if enable_file_operations {
        let plugin = fileatlas_plugin_file_operations::FileOperationsPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_file_operations::FileOperationsConfig)
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
        let plugin = fileatlas_plugin_drag_drop::DragDropPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_drag_drop::DragDropConfig)
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
        let plugin = fileatlas_plugin_metadata_export::MetadataExportPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_metadata_export::MetadataExportConfig)
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
        let plugin = fileatlas_plugin_thumbnails::ThumbnailPlugin;
        let thumbnail_config = fileatlas_plugin_thumbnails::ThumbnailConfig {
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
    let version_info_fiber = if enable_versions {
        let database = cordis
            .try_service::<fileatlas_plugin_database::DatabaseService>()
            .expect("database service is required for version metadata location");
        let path = database
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("version-info.sqlite3");
        let plugin = fileatlas_plugin_version_info::VersionInfoPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_version_info::VersionInfoConfig { path })
            .expect("prepare version-info plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start version-info plugin"),
        )
    } else {
        None
    };
    let indexing_fiber = if enable_indexing {
        let plugin = fileatlas_plugin_indexing::IndexingPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_indexing::IndexingConfig)
            .expect("prepare indexing plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start indexing plugin"),
        )
    } else {
        None
    };
    let versions_fiber = if enable_versions {
        let database = cordis
            .try_service::<fileatlas_plugin_database::DatabaseService>()
            .expect("database service is required for version storage location");
        let root = database
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("versions");
        let plugin = fileatlas_plugin_versions::VersionsPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_versions::VersionsConfig { root })
            .expect("prepare Lore versions plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start Lore versions plugin"),
        )
    } else {
        None
    };
    let watcher_fiber = if enable_watcher {
        let plugin = fileatlas_plugin_watcher::WatcherPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_watcher::WatcherConfig)
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
        let plugin = fileatlas_plugin_navigation::NavigationPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_navigation::NavigationConfig)
            .expect("prepare navigation plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start navigation plugin"),
        )
    } else {
        None
    };
    // Settings starts after Navigation and before optional workbench plugins so
    // each plugin can read its persisted launch configuration during startup.
    let settings_plugin = fileatlas_plugin_settings::SettingsPlugin;
    let settings_config = fileatlas_plugin_settings::SettingsConfig {
        path: dev_test_dir.as_ref().map(|path| path.join("settings.json")),
    };
    let settings_input = settings_plugin
        .prepare(settings_config)
        .expect("prepare settings plugin");
    let settings_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(settings_plugin, settings_input)))
        .expect("start settings plugin");
    let settings_service = cordis
        .try_service::<fileatlas_plugin_settings::SettingsService>()
        .ok();
    let material_plugin = fileatlas_plugin_material_theme::MaterialThemePlugin;
    let material_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(material_plugin, ())))
        .expect("start Material theme generator plugin");
    let plugin_runtime = settings_service
        .as_ref()
        .map(|service| service.plugin_runtime())
        .unwrap_or_default();
    let plugin_preferences = settings_service
        .as_ref()
        .and_then(|service| service.snapshot().ok())
        .unwrap_or_default();
    let enable_launcher =
        enable_launcher && plugin_preferences.plugin_is_enabled("fileatlas.launcher");
    let enable_clipboard =
        enable_clipboard && plugin_preferences.plugin_is_enabled("fileatlas.clipboard");
    let enable_screenshot =
        enable_screenshot && plugin_preferences.plugin_is_enabled("fileatlas.screenshot");
    let enable_projects =
        enable_projects && plugin_preferences.plugin_is_enabled("fileatlas.projects");
    let projects_fiber = if enable_projects {
        let plugin = fileatlas_plugin_projects::ProjectsPlugin;
        let input = match plugin.prepare(fileatlas_plugin_projects::ProjectsConfig) {
            Ok(input) => input,
            Err(never) => match never {},
        };
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start projects plugin");
                None
            }
        }
    } else {
        None
    };
    let enable_file_staging =
        enable_file_staging && plugin_preferences.plugin_is_enabled("fileatlas.file-staging");
    let file_staging_fiber = if enable_file_staging {
        let plugin = fileatlas_plugin_file_staging::FileStagingPlugin;
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ()))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start file staging plugin");
                None
            }
        }
    } else {
        None
    };
    let staging_context = cordis.clone();
    let staging_starter: fileatlas_plugin_settings::PluginStarter = Arc::new(move || {
        let context = staging_context.clone();
        Box::pin(async move {
            context
                .spawn(PreparedPlugin::from_input(
                    fileatlas_plugin_file_staging::FileStagingPlugin,
                    (),
                ))
                .await
                .map_err(|e| e.to_string())
        })
    });
    if let Err(error) = runtime.block_on(plugin_runtime.register(
        "fileatlas.file-staging",
        "File Staging Basket",
        "Collect files from different folders, then copy or move them as a batch.",
        vec![
            fileatlas_plugin_settings::PluginLaunchMode::WorkspaceAndPanel,
            fileatlas_plugin_settings::PluginLaunchMode::PanelOnly,
        ],
        file_staging_fiber,
        staging_starter,
    )) {
        tracing::error!(%error, "could not register File Staging Basket runtime control");
    }
    let projects_context = cordis.clone();
    let projects_starter: fileatlas_plugin_settings::PluginStarter = Arc::new(move || {
        let context = projects_context.clone();
        Box::pin(async move {
            let plugin = fileatlas_plugin_projects::ProjectsPlugin;
            let input = match plugin.prepare(fileatlas_plugin_projects::ProjectsConfig) {
                Ok(input) => input,
                Err(never) => match never {},
            };
            context
                .spawn(PreparedPlugin::from_input(plugin, input))
                .await
                .map_err(|error| error.to_string())
        })
    });
    if let Err(error) = runtime.block_on(plugin_runtime.register(
        "fileatlas.projects",
        "Projects",
        "Browse project folders and their complete directory trees.",
        vec![
            fileatlas_plugin_settings::PluginLaunchMode::WorkspaceAndPanel,
            fileatlas_plugin_settings::PluginLaunchMode::PanelOnly,
        ],
        projects_fiber,
        projects_starter,
    )) {
        tracing::error!(%error, "could not register Projects plugin runtime control");
    }
    let launcher_fiber = if enable_launcher {
        let plugin = fileatlas_plugin_launcher::LauncherPlugin;
        let input = match plugin.prepare(fileatlas_plugin_launcher::LauncherConfig) {
            Ok(input) => input,
            Err(never) => match never {},
        };
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start program launcher plugin");
                None
            }
        }
    } else {
        None
    };
    let launcher_context = cordis.clone();
    let launcher_starter: fileatlas_plugin_settings::PluginStarter = Arc::new(move || {
        let context = launcher_context.clone();
        Box::pin(async move {
            let plugin = fileatlas_plugin_launcher::LauncherPlugin;
            let input = match plugin.prepare(fileatlas_plugin_launcher::LauncherConfig) {
                Ok(input) => input,
                Err(never) => match never {},
            };
            context
                .spawn(PreparedPlugin::from_input(plugin, input))
                .await
                .map_err(|error| error.to_string())
        })
    });
    if let Err(error) = runtime.block_on(plugin_runtime.register(
        "fileatlas.launcher",
        "Program Launchpad",
        "Search and launch installed programs and shortcuts.",
        vec![
            fileatlas_plugin_settings::PluginLaunchMode::WorkspaceAndPanel,
            fileatlas_plugin_settings::PluginLaunchMode::PanelOnly,
        ],
        launcher_fiber,
        launcher_starter,
    )) {
        tracing::error!(%error, "could not register Program Launchpad runtime control");
    }
    let _shortcuts_fiber = if enable_shortcuts {
        let plugin = fileatlas_plugin_shortcuts::ShortcutsPlugin;
        let input = match plugin.prepare(()) {
            Ok(input) => input,
            Err(never) => match never {},
        };
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start shortcuts plugin");
                None
            }
        }
    } else {
        None
    };
    let clipboard_fiber = if enable_clipboard {
        let plugin = fileatlas_plugin_clipboard::ClipboardPlugin;
        let input = match plugin.prepare(()) {
            Ok(input) => input,
            Err(never) => match never {},
        };
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error,"could not start clipboard plugin");
                None
            }
        }
    } else {
        None
    };
    let clipboard_context = cordis.clone();
    let clipboard_starter: fileatlas_plugin_settings::PluginStarter = Arc::new(move || {
        let context = clipboard_context.clone();
        Box::pin(async move {
            let plugin = fileatlas_plugin_clipboard::ClipboardPlugin;
            let input = match plugin.prepare(()) {
                Ok(input) => input,
                Err(never) => match never {},
            };
            context
                .spawn(PreparedPlugin::from_input(plugin, input))
                .await
                .map_err(|error| error.to_string())
        })
    });
    if let Err(error) = runtime.block_on(plugin_runtime.register(
        "fileatlas.clipboard",
        "Clipboard Workbench",
        "Browse and manage captured clipboard history.",
        vec![
            fileatlas_plugin_settings::PluginLaunchMode::WorkspaceAndPanel,
            fileatlas_plugin_settings::PluginLaunchMode::PanelOnly,
        ],
        clipboard_fiber,
        clipboard_starter,
    )) {
        tracing::error!(%error, "could not register Clipboard Workbench runtime control");
    }
    let screenshot_fiber = if enable_screenshot {
        let plugin = fileatlas_plugin_screenshot::ScreenshotPlugin;
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ()))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!(%error, "could not start screenshot launcher plugin");
                None
            }
        }
    } else {
        None
    };
    let screenshot_context = cordis.clone();
    let screenshot_starter: fileatlas_plugin_settings::PluginStarter = Arc::new(move || {
        let context = screenshot_context.clone();
        Box::pin(async move {
            let plugin = fileatlas_plugin_screenshot::ScreenshotPlugin;
            let input = match plugin.prepare(()) {
                Ok(input) => input,
                Err(never) => match never {},
            };
            context
                .spawn(PreparedPlugin::from_input(plugin, input))
                .await
                .map_err(|error| error.to_string())
        })
    });
    if let Err(error) = runtime.block_on(plugin_runtime.register(
        "fileatlas.screenshot",
        "Screenshot Tool",
        "Launch Shotoshot capture and manage its output folder.",
        vec![
            fileatlas_plugin_settings::PluginLaunchMode::WorkspaceAndPanel,
            fileatlas_plugin_settings::PluginLaunchMode::PanelOnly,
            fileatlas_plugin_settings::PluginLaunchMode::Direct,
        ],
        screenshot_fiber,
        screenshot_starter,
    )) {
        tracing::error!(%error, "could not register Screenshot Tool runtime control");
    }
    // Publish the optional local Everything IPC adapter before FileManager starts.
    // The adapter never launches the external Everything application.
    let everything_fiber = if enable_file_manager {
        let plugin = fileatlas_plugin_everything_search::EverythingSearchPlugin;
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ())))
                .expect("start Everything search adapter"),
        )
    } else {
        None
    };
    // Start AI after Settings so future consumers can read the user's
    // mandatory tag groups before requesting suggestions.
    let ai_fiber = if enable_ai {
        let plugin = fileatlas_plugin_ai::AiPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_ai::AiConfig)
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
        let plugin = fileatlas_plugin_tagging::TaggingPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_tagging::TaggingConfig)
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
        let plugin = fileatlas_plugin_history::HistoryPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_history::HistoryConfig)
            .expect("prepare history plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start history plugin"),
        )
    } else {
        None
    };
    let quicklook_fiber = if enable_file_manager {
        let plugin = fileatlas_plugin_quicklook::QuickLookPlugin;
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ())))
                .expect("start QuickLook plugin"),
        )
    } else {
        None
    };
    let grouping_fiber = if enable_file_manager {
        let plugin = fileatlas_plugin_grouping::GroupingPlugin;
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, ())))
                .expect("start grouping plugin"),
        )
    } else {
        None
    };
    let file_manager_fiber = if enable_file_manager {
        let plugin = fileatlas_plugin_file_manager::FileManagerPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_file_manager::FileManagerConfig)
            .expect("prepare file_manager plugin");
        Some(
            runtime
                .block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input)))
                .expect("start file_manager plugin"),
        )
    } else {
        None
    };
    let dev_test_fiber = if enable_dev_test {
        let plugin = fileatlas_plugin_dev_test::DevTestPlugin;
        let input = plugin
            .prepare(fileatlas_plugin_dev_test::DevTestConfig)
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
        let plugin = if mcp_arg {
            fileatlas_plugin_mcp::McpPlugin::stdio()
        } else {
            fileatlas_plugin_mcp::McpPlugin::in_process()
        };
        let input = plugin.prepare(()).expect("prepare MCP plugin");
        match runtime.block_on(cordis.spawn(PreparedPlugin::from_input(plugin, input))) {
            Ok(fiber) => Some(fiber),
            Err(error) => {
                tracing::error!("MCP endpoint failed to start; continuing without MCP: {error}");
                eprintln!("MCP endpoint failed to start; continuing without MCP: {error}");
                None
            }
        }
    } else {
        None
    };
    let executor = runtime.handle().clone();
    let app_cordis = cordis.clone();

    let application = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets);
    application.run(move |cx| {
        gpui_kit::init(cx);
        if let Ok(shortcuts) =
            app_cordis.try_service::<fileatlas_plugin_shortcuts::ShortcutService>()
        {
            let overrides = app_cordis
                .try_service::<fileatlas_plugin_settings::SettingsService>()
                .ok()
                .and_then(|service| service.snapshot().ok())
                .map(|settings| settings.keybindings)
                .unwrap_or_default();
            if let Err(error) = shortcuts.start(&overrides) {
                tracing::error!(%error, "could not register global shortcuts");
            }
        }
        if let Ok(i18n) = app_cordis.try_service::<fileatlas_plugin_i18n::I18nService>() {
            gpui_kit::component::set_locale(i18n.language().code());
        }
        fileatlas_ui_bridge::theme::apply_components(cx);
        let catalog = Arc::clone(&catalog);
        let views = Arc::clone(&bridge.views);
        let cordis = app_cordis.clone();
        let executor = executor.clone();
        let capture_handle = Arc::clone(&capture_handle);
        let session = cordis
            .try_service::<fileatlas_plugin_session::SessionService>()
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
        let work_area = cx
            .primary_display()
            .map(|display| display.visible_bounds().size)
            .map(|area| (f32::from(area.width), f32::from(area.height)));
        let (window_width, window_height) = startup_window_extent(work_area);
        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(
                size(px(window_width), px(window_height)),
                cx,
            )),
            window_min_size: Some(size(
                px(theme::WINDOW_MIN_WIDTH),
                px(theme::WINDOW_MIN_HEIGHT),
            )),
            ..WindowOptions::default()
        };
        gpui_kit::open_window(window_options, cx, move |window, cx| {
            window.set_window_title("FileAtlas");
            #[cfg(target_os = "windows")]
            let native_window_handle = {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                window
                    .window_handle()
                    .ok()
                    .and_then(|handle| match handle.as_raw() {
                        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as isize),
                        _ => None,
                    })
                    .unwrap_or_default()
            };
            #[cfg(not(target_os = "windows"))]
            let native_window_handle = 0;
            let _ = bridge
                .surfaces
                .set_value("mcp.window.main", &native_window_handle.to_string());
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
                    .map(fileatlas_ui_bridge::winfx::apply_backdrop)
                    .unwrap_or(fileatlas_ui_bridge::winfx::Backdrop::None);
                fileatlas_ui_bridge::theme::set_backdrop_active(backdrop.is_translucent());
            }
            #[cfg(not(target_os = "windows"))]
            {
                capture_handle.set(1);
                fileatlas_ui_bridge::theme::set_backdrop_active(false);
            }
            let view_snapshot = views.snapshot().unwrap_or_default();
            let view_catalog_signature = view_snapshot
                .iter()
                .map(|entry| (entry.contribution.id.clone(), entry.generation))
                .collect::<Vec<_>>();
            let mounted_views = view_snapshot
                .into_iter()
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
                let mut locale = fileatlas_ui_bridge::refresh::ChangeGate::new(
                    fileatlas_ui_bridge::i18n::revision(),
                );
                fileatlas_ui_bridge::refresh::poll(
                    cx,
                    std::time::Duration::from_millis(100),
                    move |shell: &mut FileAtlasShell, cx| {
                        let mut changed = locale.changed(fileatlas_ui_bridge::i18n::revision());
                        if changed {
                            if let Ok(i18n) = shell
                                .cordis
                                .try_service::<fileatlas_plugin_i18n::I18nService>()
                            {
                                gpui_kit::component::set_locale(i18n.language().code());
                            }
                            cx.refresh_windows();
                        }
                        shell.poll_panel_requests(cx);
                        shell.sync_native_panels(cx);
                        if let Ok(shortcuts) = shell
                            .cordis
                            .try_service::<fileatlas_plugin_shortcuts::ShortcutService>(
                        ) {
                            let overrides = shell
                                .cordis
                                .try_service::<fileatlas_plugin_settings::SettingsService>()
                                .ok()
                                .and_then(|service| service.snapshot().ok())
                                .map(|settings| settings.keybindings)
                                .unwrap_or_default();
                            if shortcuts.poll(
                                shell.native_window_handle,
                                &overrides,
                                shell.surfaces.is_capturing_key("settings"),
                            ) {
                                cx.refresh_windows();
                                changed = true;
                            }
                        }
                        let registered = shell
                            .view_catalog
                            .snapshot()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|entry| (entry.contribution.id, entry.generation))
                            .collect::<Vec<_>>();
                        if registered != shell.view_catalog_signature {
                            shell.view_catalog_signature = registered;
                            if let Ok(navigation) = shell
                                .cordis
                                .try_service::<fileatlas_plugin_navigation::NavigationService>(
                            ) {
                                navigation.notify_plugin_views_changed();
                            }
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
                FileAtlasShell {
                    catalog,
                    views: mounted_views,
                    view_catalog_signature,
                    view_catalog: views.clone(),
                    cordis,
                    executor,
                    session: session_for_shell,
                    surfaces: Arc::clone(&bridge.surfaces),
                    sidebar_width,
                    inspector_width,
                    viewport_width: window_width,
                    capture: Arc::clone(&capture_handle),
                    native_window_handle,
                    drag_origin_x: 0.,
                    drag_origin_width: 0.,
                    resizing_panel: None,
                    tools_open: false,
                    sidebar_open: true,
                    inspector_open: true,
                    native_panels: Default::default(),
                    panel_signature: (false, None),
                    column_refresh_task: None,
                    column_revision: 0,
                }
            })
        })
        .expect("open FileAtlas window");
    });

    if let Ok(updater) = cordis.try_service::<fileatlas_plugin_updater::UpdaterService>() {
        match updater.take_pending_install() {
            Ok(Some(staged)) => {
                if let Err(error) = updater.install(&staged) {
                    let log = dirs::data_local_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("FileAtlas")
                        .join("update-error.txt");
                    let _ = std::fs::create_dir_all(
                        log.parent().unwrap_or_else(|| std::path::Path::new(".")),
                    );
                    let _ = std::fs::write(
                        &log,
                        format!(
                            "FileAtlas {} could not install update from {}\n\n{}\n",
                            fileatlas_plugin_updater::current_version(),
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
    if let Some(fiber) = file_manager_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop file_manager plugin");
    }
    if let Some(fiber) = everything_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop Everything search adapter");
    }
    if let Some(fiber) = versions_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop Lore versions plugin");
    }
    if let Some(fiber) = version_info_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop version-info plugin");
    }
    if let Some(fiber) = grouping_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop grouping plugin");
    }
    if let Some(fiber) = quicklook_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop QuickLook plugin");
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
        .block_on(material_fiber.dispose())
        .expect("stop Material theme generator plugin");
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

#[cfg(test)]
mod tests {
    use super::startup_window_extent;

    #[test]
    fn uses_a_share_of_a_large_work_area() {
        let (width, height) = startup_window_extent(Some((1920., 1200.)));
        assert_eq!((width, height), (1632., 1020.));
    }

    #[test]
    fn never_drops_below_the_design_baseline() {
        let (width, height) = startup_window_extent(Some((1366., 768.)));
        assert_eq!((width, height), (1280., 768.));
    }

    #[test]
    fn never_exceeds_the_available_area() {
        let (width, height) = startup_window_extent(Some((1000., 700.)));
        assert_eq!((width, height), (1000., 700.));
    }

    #[test]
    fn falls_back_to_the_baseline_without_a_display() {
        let (width, height) = startup_window_extent(None);
        assert_eq!((width, height), (1280., 800.));
    }
}
