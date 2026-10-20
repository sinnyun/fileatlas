use super::*;

impl WorkCratePanel {
    pub(super) fn perform_export(
        &mut self,
        job: impl FnOnce() -> Result<(), String> + Send + 'static,
        cx: &mut ViewContext<Self>,
    ) {
        if self.busy {
            self.message = Some("请等待当前操作完成".into());
            cx.notify();
            return;
        }
        self.busy = true;
        self.message = None;
        self.export_status = Some("准备导出…".into());
        cx.notify();
        fileatlas_ui_bridge::refresh::poll(
            cx,
            std::time::Duration::from_millis(100),
            |view: &mut Self, _| {
                let active = view.export_status.is_some();
                let next = if active {
                    view.exports.progress()
                } else {
                    None
                };
                let changed = next.is_some() && next != view.export_status;
                if changed {
                    view.export_status = next;
                }
                fileatlas_ui_bridge::refresh::PollResult::until(changed, active)
            },
        );
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { job() }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.export_status = None;
                view.message = Some(match result {
                    Ok(()) => view.exports.progress().unwrap_or_else(|| "导出完成".into()),
                    Err(e) => format!("导出失败：{e}"),
                });
                // Export does not alter the document: never reload or re-decode its images.
                cx.notify();
            });
        })
        .detach();
    }
}
