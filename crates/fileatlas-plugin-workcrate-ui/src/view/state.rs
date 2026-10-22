use super::*;
#[derive(Default)]
pub(crate) struct WorkCrateUiState {
    connected: std::sync::atomic::AtomicBool,
    selected_board: std::sync::RwLock<Option<(String, String)>>,
    wake: fileatlas_ui_bridge::refresh::Wake,
    board_refresh_revision: std::sync::atomic::AtomicU64,
    catalog_refresh_revision: std::sync::atomic::AtomicU64,
    order_refresh: std::sync::Mutex<Option<fileatlas_plugin_workcrate_boards::PageOrderSave>>,
}

impl Service for WorkCrateUiState {
    const NAME: &'static str = "fileatlas/workcrate-ui-state";
}

impl WorkCrateUiState {
    pub fn publish_order(
        &self,
        save: fileatlas_plugin_workcrate_boards::PageOrderSave,
    ) -> Result<(), String> {
        *self
            .order_refresh
            .lock()
            .map_err(|_| "页面排序通知不可用")? = Some(save);
        self.wake.trigger();
        Ok(())
    }
    pub(super) fn take_order_refresh(
        &self,
    ) -> Option<fileatlas_plugin_workcrate_boards::PageOrderSave> {
        self.order_refresh.lock().ok()?.take()
    }
    pub fn set_connection(&self, value: bool) {
        self.connected
            .store(value, std::sync::atomic::Ordering::Release);
        self.wake.trigger();
    }
    pub fn connection(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Acquire)
    }
    pub fn selected_board(&self) -> Option<(String, String)> {
        self.selected_board
            .read()
            .ok()
            .and_then(|value| value.clone())
    }

    pub fn select_board(&self, project_id: String, board_id: String) -> Result<(), String> {
        *self
            .selected_board
            .write()
            .map_err(|_| "WorkCrate UI selection is unavailable")? = Some((project_id, board_id));
        self.wake.trigger();
        Ok(())
    }

    pub fn wake(&self) -> fileatlas_ui_bridge::refresh::Wake {
        self.wake.clone()
    }

    pub fn request_board_refresh(&self) {
        self.board_refresh_revision
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.wake.trigger();
    }

    pub fn request_catalog_refresh(&self) {
        self.catalog_refresh_revision
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.wake.trigger();
    }

    pub(super) fn board_refresh_revision(&self) -> u64 {
        self.board_refresh_revision
            .load(std::sync::atomic::Ordering::Acquire)
    }

    pub(super) fn catalog_refresh_revision(&self) -> u64 {
        self.catalog_refresh_revision
            .load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn navigate(&self, project: Option<String>, board: Option<String>) -> Result<(), String> {
        self.select_board(project.unwrap_or_default(), board.unwrap_or_default())
    }
}
