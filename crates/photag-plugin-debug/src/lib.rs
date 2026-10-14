//! Runtime diagnostics owned by a compile-time Cordis plugin.
mod capture;
mod store;
mod view;
pub use capture::attach;
use cordis::{Context, Plugin, event::observer_sync};
use photag_ui_bridge::{
    ContributionKind, UiBridgeService, UiCommandInvoked, UiContribution, register_for_plugin,
    register_views_for_plugin,
};
use std::sync::Arc;
pub use store::{DebugService, Filter, Page, Record, UiState};

pub struct DebugPlugin;
impl Plugin for DebugPlugin {
    type Config = Arc<DebugService>;
    type Input = Arc<DebugService>;
    type PrepareError = std::convert::Infallible;
    type ApplyError = std::io::Error;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.debug".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    async fn apply(&self, context: Context, service: &Self::Input) -> Result<(), Self::ApplyError> {
        let error = |e: String| std::io::Error::other(e);
        context
            .try_service::<UiBridgeService>()
            .map_err(|e| error(e.to_string()))?;
        capture::attach(service).map_err(error)?;
        let cleanup = service.clone();
        context
            .effect_sync(move || capture::detach(&cleanup))
            .map_err(|e| error(e.to_string()))?;
        let _publication = context
            .provide(service.clone())
            .map_err(|e| error(e.to_string()))?;
        register_for_plugin(
            &context,
            "photag.debug",
            [UiContribution {
                id: "debug/panel".into(),
                kind: ContributionKind::Widget {
                    slot: "workspace.bottom".into(),
                    label: "Runtime diagnostics".into(),
                    order: 100,
                },
            }],
        )
        .map_err(|e| error(e.to_string()))?;
        register_views_for_plugin(&context, "photag.debug", view::contributions(service))
            .map_err(|e| error(e.to_string()))?;
        let sink = service.clone();
        context
            .on::<UiCommandInvoked, _>(observer_sync(move |_, command: String| {
                sink.record(
                    "INFO",
                    "photag_ui_bridge",
                    "command",
                    &command,
                    None,
                    None,
                    None,
                );
                Ok::<_, std::convert::Infallible>(())
            }))
            .map_err(|e| error(e.to_string()))?;
        macro_rules! observe {
            ($event:ty,$action:literal) => {{
                let sink = service.clone();
                context
                    .on::<$event, _>(observer_sync(move |_, args| {
                        sink.record(
                            "INFO",
                            &format!(
                                "photag_{}",
                                <$event as cordis::Event>::NAME
                                    .split('/')
                                    .nth(1)
                                    .unwrap_or("events")
                            ),
                            $action,
                            &format!("{args:?}"),
                            None,
                            None,
                            None,
                        );
                        Ok::<_, std::convert::Infallible>(())
                    }))
                    .map_err(|e| error(e.to_string()))?;
            }};
        }
        observe!(photag_contracts::FolderSelectionChanged, "folder_selection");
        observe!(
            photag_contracts::DirectoryPathSelected,
            "directory_selection"
        );
        observe!(photag_contracts::TagSelectionChanged, "tag_selection");
        observe!(photag_contracts::TagFilterSelectionChanged, "tag_filter");
        observe!(photag_contracts::MediaIndexChanged, "media_index_changed");
        service.record(
            "INFO",
            "photag_debug",
            "startup",
            "Runtime diagnostics active; bounded to 10000 records",
            None,
            None,
            None,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn plugin_disposal_removes_services_views_and_event_observers() {
        let context = Context::new();
        let bridge = Arc::new(UiBridgeService::new(Arc::default()));
        let _bridge = context.provide(bridge.clone()).unwrap();
        let service = Arc::new(DebugService::default());
        let fiber = context
            .spawn(cordis::PreparedPlugin::from_input(
                DebugPlugin,
                service.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(bridge.views.snapshot().unwrap().len(), 2);
        context
            .emit::<UiCommandInvoked>(cordis::Routing::Unscoped, "test-command".into())
            .await
            .unwrap();
        assert!(
            service
                .query(&Filter::default())
                .records
                .iter()
                .any(|r| r.message == "test-command")
        );
        fiber.dispose().await.unwrap();
        assert!(context.try_service::<DebugService>().is_err());
        assert!(bridge.views.snapshot().unwrap().is_empty());
        assert!(bridge.catalog.snapshot().unwrap().is_empty());
        let before = service.revision();
        context
            .emit::<UiCommandInvoked>(cordis::Routing::Unscoped, "after-stop".into())
            .await
            .unwrap();
        tracing::info!("after stop");
        assert_eq!(before, service.revision());
    }
}
