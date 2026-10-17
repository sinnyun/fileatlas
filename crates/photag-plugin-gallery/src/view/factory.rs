//! View-contribution factory: builds the shared GalleryView entity once.

use super::*;

pub(crate) fn gallery(
    service: Arc<GalleryService>,
    file_operations: Arc<FileOperationsService>,
    drag_drop: Arc<DragDropService>,
    quicklook: Arc<QuickLookService>,
    versions: Arc<photag_plugin_versions::VersionsService>,
    runtime: tokio::runtime::Handle,
    cordis: CordisContext,
    selected_folder: Arc<SelectedFolder>,
    selected_tag: Arc<SelectedTag>,
    selected_path: Arc<SelectedPath>,
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
    history: Option<Arc<HistoryService>>,
    settings: Option<Arc<SettingsService>>,
    tagging: Option<Arc<TaggingService>>,
    surfaces: Arc<photag_ui_bridge::UiSurfaceState>,
    ui_views: Arc<photag_ui_bridge::UiViewCatalog>,
) -> Vec<UiViewContribution> {
    // All regions share one entity. A filter or selection change invalidates
    // every projection; no independent state, workers or session writers.
    let cached = Arc::new(std::sync::Mutex::new(None::<Entity<GalleryView>>));
    let create = Arc::new(move |window: &mut Window, app: &mut App| {
        let mut cached = cached.lock().expect("Gallery factory lock");
        if let Some(view) = cached.as_ref() {
            return view.clone();
        }
        let inputs = view_state::build_inputs(window, app, &session);
        let view = app.new(|cx| {
            GalleryView::new(
                Arc::clone(&service),
                Arc::clone(&file_operations),
                Arc::clone(&drag_drop),
                Arc::clone(&quicklook),
                Arc::clone(&versions),
                runtime.clone(),
                cordis.clone(),
                Arc::clone(&selected_folder),
                Arc::clone(&selected_tag),
                Arc::clone(&selected_path),
                Arc::clone(&index_revision),
                Arc::clone(&session),
                history.clone(),
                settings.clone(),
                tagging.clone(),
                Arc::clone(&surfaces),
                Arc::clone(&ui_views),
                inputs,
                cx,
            )
        });
        filter_row::subscribe_calendar(&view, window, app);
        view.update(app, |view, cx| monitor::spawn(view, cx));
        *cached = Some(view.clone());
        view
    });
    regions::contributions(create)
}
