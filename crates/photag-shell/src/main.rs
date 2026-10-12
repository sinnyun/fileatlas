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
    ];
    for plugin in config
        .split(',')
        .map(str::trim)
        .filter(|plugin| !plugin.is_empty())
    {
        if !valid.contains(&plugin) {
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
    ContributionKind, OwnedContribution, UiBridgeService, UiCatalog, UiCommandInvoked,
};

struct PhoTagShell {
    catalog: Arc<UiCatalog>,
    views: Vec<(String, gpui_kit::AnyView)>,
    cordis: CordisContext,
    executor: tokio::runtime::Handle,
}

impl Render for PhoTagShell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let contributions = self.catalog.snapshot().unwrap_or_default();
        let mut sidebar_views = Vec::new();
        let mut content_views = Vec::new();
        let mut auxiliary_views = Vec::new();
        for (slot, view) in &self.views {
            match slot.as_str() {
                "workspace.sidebar" => sidebar_views.push(view.clone()),
                "gallery.content" | "workspace.main" => content_views.push(view.clone()),
                _ => auxiliary_views.push(view.clone()),
            }
        }
        let commands = contributions.iter().filter_map(|owned| {
            let ContributionKind::Command { label } = &owned.contribution.kind else {
                return None;
            };
            let command_id = owned.contribution.id.clone();
            let cordis = self.cordis.clone();
            let executor = self.executor.clone();
            Some(
                Button::new(command_id.clone())
                    .label(label.clone())
                    .on_click(move |_, _, _| {
                        let cordis = cordis.clone();
                        let command_id = command_id.clone();
                        executor.spawn(async move {
                            if let Err(error) = cordis
                                .emit::<UiCommandInvoked>(Routing::Unscoped, command_id)
                                .await
                            {
                                eprintln!("command dispatch failed: {error}");
                            }
                        });
                    }),
            )
        });
        let mut shell = div()
            .v_flex()
            .size_full()
            .bg(rgb(0x181a20))
            .text_color(rgb(0xe7e9ee))
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .bg(rgb(0x20232b))
                    .child(div().text_lg().font_semibold().child("PhoTag"))
                    .children(commands),
            )
            .child(
                div()
                    .h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .v_flex()
                            .w_72()
                            .border_r_1()
                            .border_color(rgb(0x343943))
                            .overflow_y_scrollbar()
                            .children(sidebar_views),
                    )
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .overflow_y_scrollbar()
                            .children(content_views)
                            .children(auxiliary_views),
                    ),
            );
        if self.views.is_empty() {
            shell = shell.child(
                div()
                    .v_flex()
                    .flex_1()
                    .gap_2()
                    .overflow_y_scrollbar()
                    .p_4()
                    .children(contributions.iter().map(render_contribution))
                    .when(contributions.is_empty(), |body| {
                        body.child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x9aa0ad))
                                .child("No feature plugins are active."),
                        )
                    }),
            );
        }
        shell
    }
}

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
        .border_color(rgb(0x343943))
        .bg(rgb(0x20232b))
        .px_3()
        .py_2()
        .child(
            div()
                .w_24()
                .text_xs()
                .text_color(rgb(0x9aa0ad))
                .child(category),
        )
        .child(div().flex_1().child(title))
        .child(div().text_xs().text_color(rgb(0x9aa0ad)).child(location))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x747b89))
                .child(owned.owner.clone()),
        )
}

fn main() {
    let plugin_config = dev_plugin_config();
    let enable_all = dev_plugin_enabled(&plugin_config, "all");
    let enable_history = enable_all || dev_plugin_enabled(&plugin_config, "history");
    let enable_gallery =
        enable_all || enable_history || dev_plugin_enabled(&plugin_config, "gallery");
    let enable_navigation = enable_all || dev_plugin_enabled(&plugin_config, "navigation");
    let enable_metadata_export =
        enable_all || dev_plugin_enabled(&plugin_config, "metadata-export");
    let enable_file_operations =
        enable_all || enable_gallery || dev_plugin_enabled(&plugin_config, "file-operations");
    let enable_thumbnails = enable_all
        || enable_navigation
        || dev_plugin_enabled(&plugin_config, "thumbnails")
        || dev_plugin_enabled(&plugin_config, "indexing")
        || dev_plugin_enabled(&plugin_config, "watcher");
    let enable_indexing = enable_all
        || enable_navigation
        || dev_plugin_enabled(&plugin_config, "indexing")
        || dev_plugin_enabled(&plugin_config, "watcher");
    let enable_watcher =
        enable_all || enable_navigation || dev_plugin_enabled(&plugin_config, "watcher");
    let enable_tagging =
        enable_all || enable_history || dev_plugin_enabled(&plugin_config, "tagging");
    let enable_history = enable_history || (enable_gallery && enable_tagging);
    let enable_media_playback =
        enable_all || enable_gallery || dev_plugin_enabled(&plugin_config, "media-playback");
    let enable_drag_drop =
        enable_all || enable_gallery || dev_plugin_enabled(&plugin_config, "drag-drop");
    eprintln!("Starting development plugins: {plugin_config}");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create Cordis async runtime");
    let cordis = CordisContext::new();
    let catalog = Arc::new(UiCatalog::default());
    let bridge = Arc::new(UiBridgeService::new(Arc::clone(&catalog)));
    let _bridge_publication = cordis
        .provide(Arc::clone(&bridge))
        .expect("publish UI bridge service");

    let session_plugin = photag_plugin_session::SessionPlugin;
    let session_input = session_plugin
        .prepare(photag_plugin_session::SessionConfig::default())
        .expect("prepare session plugin");
    let session_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(session_plugin, session_input)))
        .expect("start session plugin");
    let database_plugin = photag_plugin_database::DatabasePlugin;
    let database_input = database_plugin
        .prepare(photag_plugin_database::DatabaseConfig::default())
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
    let thumbnail_fiber = if enable_thumbnails {
        let plugin = photag_plugin_thumbnails::ThumbnailPlugin;
        let input = plugin
            .prepare(photag_plugin_thumbnails::ThumbnailConfig::default())
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
    let settings_input = settings_plugin
        .prepare(photag_plugin_settings::SettingsConfig::default())
        .expect("prepare settings plugin");
    let settings_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(settings_plugin, settings_input)))
        .expect("start settings plugin");
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
    let executor = runtime.handle().clone();

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let catalog = Arc::clone(&catalog);
        let views = Arc::clone(&bridge.views);
        let cordis = cordis.clone();
        let executor = executor.clone();
        gpui_kit::open_window(WindowOptions::default(), cx, move |window, cx| {
            #[cfg(target_os = "windows")]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let backdrop = window
                    .window_handle()
                    .ok()
                    .and_then(|handle| match handle.as_raw() {
                        RawWindowHandle::Win32(handle) => Some(
                            photag_ui_bridge::winfx::apply_backdrop(handle.hwnd.get() as isize),
                        ),
                        _ => None,
                    })
                    .unwrap_or(photag_ui_bridge::winfx::Backdrop::None);
                photag_ui_bridge::theme::set_backdrop_active(backdrop.is_translucent());
            }
            #[cfg(not(target_os = "windows"))]
            photag_ui_bridge::theme::set_backdrop_active(false);
            let mounted_views = views
                .snapshot()
                .unwrap_or_default()
                .into_iter()
                .map(|entry| {
                    let slot = entry.contribution.slot.clone();
                    let view = (entry.contribution.factory)(window, cx);
                    (slot, view)
                })
                .collect();
            cx.new(|_| PhoTagShell {
                catalog,
                views: mounted_views,
                cordis,
                executor,
            })
        })
        .expect("open PhoTag window");
    });

    if let Some(fiber) = history_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop history plugin");
    }
    if let Some(fiber) = media_playback_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop media playback plugin");
    }
    if let Some(fiber) = tagging_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop tagging plugin");
    }
    if let Some(fiber) = gallery_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop gallery plugin");
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
    if let Some(fiber) = preview_fiber {
        runtime
            .block_on(fiber.dispose())
            .expect("stop image preview plugin");
    }
    runtime
        .block_on(database_fiber.dispose())
        .expect("stop database plugin");
    runtime
        .block_on(session_fiber.dispose())
        .expect("stop session plugin");
    runtime
        .block_on(settings_fiber.dispose())
        .expect("stop settings plugin");
}
