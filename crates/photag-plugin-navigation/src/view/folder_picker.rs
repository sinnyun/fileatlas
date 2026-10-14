//! Async native picker integration; avoids re-entering a borrowed GPUI app.
use super::*;

impl SidebarView {
    pub(super) fn pick_folder(&mut self, cx: &mut Context<Self>) {
        // Match PhoTag's PendingAction::AddFolder flow: a native directory
        // dialog returns an optional path, and cancellation is a no-op.
        let title = format!("PhoTag · {}", photag_ui_bridge::i18n::tr("Add folder"));
        let Some(lease) = self.service.picker.begin(title.clone()) else {
            return;
        };
        self.folder_picker_task = Some(cx.spawn(async move |this, cx| {
            // rfd runs the Windows modal loop on its own STA thread. Awaiting here
            // releases the GPUI entity/app borrow before native messages arrive.
            let path = rfd::AsyncFileDialog::new()
                .set_title(title)
                .pick_folder()
                .await;
            drop(lease);
            let _ = this.update(cx, |view, cx| {
                if let Some(path) = path {
                    view.add_folder_path(path.path().to_string_lossy().into_owned(), cx);
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
}
