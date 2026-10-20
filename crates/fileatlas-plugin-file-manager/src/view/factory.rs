//! View-contribution factory: creates isolated FileManagerView entities per pane.

use super::*;

pub(crate) fn file_manager(
    services: [Arc<FileManagerService>; 4],
    file_operations: Arc<FileOperationsService>,
    drag_drop: Arc<DragDropService>,
    quicklook: Arc<QuickLookService>,
    versions: Arc<fileatlas_plugin_versions::VersionsService>,
    runtime: tokio::runtime::Handle,
    cordis: CordisContext,
    selected_folders: [Arc<SelectedFolder>; 4],
    selected_tags: [Arc<SelectedTag>; 4],
    selected_paths: [Arc<SelectedPath>; 4],
    index_revision: Arc<IndexRevision>,
    session: Arc<SessionService>,
    history: Option<Arc<HistoryService>>,
    settings: Option<Arc<SettingsService>>,
    tagging: Option<Arc<TaggingService>>,
    surfaces: Arc<fileatlas_ui_bridge::UiSurfaceState>,
    ui_views: Arc<fileatlas_ui_bridge::UiViewCatalog>,
    workspaces: Arc<fileatlas_plugin_workspaces::WorkspaceService>,
) -> Vec<UiViewContribution> {
    let mut contributions = Vec::new();
    for column in 0..4 {
        let column_service = Arc::clone(&services[column]);
        let folder = Arc::clone(&selected_folders[column]);
        let tag = Arc::clone(&selected_tags[column]);
        let path = Arc::clone(&selected_paths[column]);
        let index_revision = Arc::clone(&index_revision);
        let session = Arc::clone(&session);
        let file_operations = Arc::clone(&file_operations);
        let drag_drop = Arc::clone(&drag_drop);
        let quicklook = Arc::clone(&quicklook);
        let versions = Arc::clone(&versions);
        let runtime = runtime.clone();
        let cordis = cordis.clone();
        let history = history.clone();
        let settings = settings.clone();
        let tagging = tagging.clone();
        let surfaces = Arc::clone(&surfaces);
        let ui_views = Arc::clone(&ui_views);
        let workspaces = Arc::clone(&workspaces);
        let cached = Arc::new(std::sync::Mutex::new(None::<Entity<FileManagerView>>));
        let create = Arc::new(move |window: &mut Window, app: &mut App| {
            let mut cached = cached.lock().expect("FileManager factory lock");
            if let Some(view) = cached.as_ref() {
                return view.clone();
            }
            let initial_workspace_column = workspaces
                .active()
                .ok()
                .and_then(|workspace| workspace.columns.get(column).cloned())
                .unwrap_or_default();
            let inputs = view_state::build_inputs(window, app, &session, &initial_workspace_column);
            let workspace_for_view = Arc::clone(&workspaces);
            let view = app.new(|cx| {
                FileManagerView::new(
                    Arc::clone(&column_service),
                    Arc::clone(&file_operations),
                    Arc::clone(&drag_drop),
                    Arc::clone(&quicklook),
                    Arc::clone(&versions),
                    runtime.clone(),
                    cordis.clone(),
                    Arc::clone(&folder),
                    Arc::clone(&tag),
                    Arc::clone(&path),
                    Arc::clone(&index_revision),
                    Arc::clone(&session),
                    history.clone(),
                    settings.clone(),
                    tagging.clone(),
                    Arc::clone(&surfaces),
                    Arc::clone(&ui_views),
                    column,
                    initial_workspace_column,
                    inputs,
                    cx,
                )
            });
            filter_row::subscribe_calendar(&view, window, app);
            view.update(app, |view, cx| {
                monitor::spawn(view, cx);
                view.watch_workspace(workspace_for_view, cx);
            });
            *cached = Some(view.clone());
            view
        });
        contributions.extend(regions::column_contributions(create, column));
    }
    contributions
}
