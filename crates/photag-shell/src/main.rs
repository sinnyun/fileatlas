//! Native GPUI host and Cordis runtime composition.

use gpui_kit::prelude::FluentBuilder;

use std::sync::Arc;

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
    cordis: CordisContext,
    executor: tokio::runtime::Handle,
}

impl Render for PhoTagShell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let contributions = self.catalog.snapshot().unwrap_or_default();
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
        let extension_rows = contributions.iter().map(render_contribution);

        div()
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
                    .v_flex()
                    .flex_1()
                    .gap_2()
                    .overflow_y_scrollbar()
                    .p_4()
                    .children(extension_rows)
                    .when(contributions.is_empty(), |body| {
                        body.child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x9aa0ad))
                                .child("No feature plugins are active."),
                        )
                    }),
            )
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
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create Cordis async runtime");
    let cordis = CordisContext::new();
    let catalog = Arc::new(UiCatalog::default());
    let _bridge_publication = cordis
        .provide(Arc::new(UiBridgeService::new(Arc::clone(&catalog))))
        .expect("publish UI bridge service");

    let database_plugin = photag_plugin_database::DatabasePlugin;
    let database_input = database_plugin
        .prepare(photag_plugin_database::DatabaseConfig::default())
        .expect("prepare database plugin");
    let database_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(database_plugin, database_input)))
        .expect("start database plugin");
    let gallery_plugin = photag_plugin_gallery::GalleryPlugin;
    let gallery_input = gallery_plugin
        .prepare(photag_plugin_gallery::GalleryConfig)
        .expect("prepare gallery plugin");
    let gallery_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(gallery_plugin, gallery_input)))
        .expect("start gallery plugin");
    let thumbnail_plugin = photag_plugin_thumbnails::ThumbnailPlugin;
    let thumbnail_input = thumbnail_plugin
        .prepare(photag_plugin_thumbnails::ThumbnailConfig::default())
        .expect("prepare thumbnail plugin");
    let thumbnail_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(
            thumbnail_plugin,
            thumbnail_input,
        )))
        .expect("start thumbnail plugin");
    let indexing_plugin = photag_plugin_indexing::IndexingPlugin;
    let indexing_input = indexing_plugin
        .prepare(photag_plugin_indexing::IndexingConfig)
        .expect("prepare indexing plugin");
    let indexing_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(indexing_plugin, indexing_input)))
        .expect("start indexing plugin");
    let watcher_plugin = photag_plugin_watcher::WatcherPlugin;
    let watcher_input = watcher_plugin
        .prepare(photag_plugin_watcher::WatcherConfig)
        .expect("prepare watcher plugin");
    let watcher_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(watcher_plugin, watcher_input)))
        .expect("start watcher plugin");
    let navigation_plugin = photag_plugin_navigation::NavigationPlugin;
    let navigation_input = navigation_plugin
        .prepare(photag_plugin_navigation::NavigationConfig)
        .expect("prepare navigation plugin");
    let navigation_fiber = runtime
        .block_on(cordis.spawn(PreparedPlugin::from_input(
            navigation_plugin,
            navigation_input,
        )))
        .expect("start navigation plugin");
    let executor = runtime.handle().clone();

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        let catalog = Arc::clone(&catalog);
        let cordis = cordis.clone();
        let executor = executor.clone();
        gpui_kit::open_window(WindowOptions::default(), cx, move |_, cx| {
            cx.new(|_| PhoTagShell {
                catalog,
                cordis,
                executor,
            })
        })
        .expect("open PhoTag window");
    });

    runtime
        .block_on(navigation_fiber.dispose())
        .expect("stop navigation plugin");
    runtime
        .block_on(watcher_fiber.dispose())
        .expect("stop watcher plugin");
    runtime
        .block_on(indexing_fiber.dispose())
        .expect("stop indexing plugin");
    runtime
        .block_on(thumbnail_fiber.dispose())
        .expect("stop thumbnail plugin");
    runtime
        .block_on(gallery_fiber.dispose())
        .expect("stop gallery plugin");
    runtime
        .block_on(database_fiber.dispose())
        .expect("stop database plugin");
}
