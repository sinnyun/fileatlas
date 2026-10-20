//! Async native picker integration; avoids re-entering a borrowed GPUI app.
use super::*;

impl SidebarView {
    pub(super) fn pick_folder(&mut self, cx: &mut Context<Self>) {
        // Match FileAtlas's PendingAction::AddFolder flow: a native directory
        // dialog returns an optional path, and cancellation is a no-op.
        let title = format!(
            "FileAtlas · {}",
            fileatlas_ui_bridge::i18n::tr("Add folder")
        );
        let editor = self.picker_editor.clone();
        let editor_for_task = editor.clone();
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
                    let path = path.path().to_string_lossy().into_owned();
                    match editor_for_task {
                        Some(SidebarEditor::AddQuickAccess) => {
                            view.add_quick_access_path(path, cx);
                        }
                        Some(SidebarEditor::AddFavorite { group_id }) => {
                            view.add_favorite_folder_path(group_id, path, cx);
                        }
                        None => view.add_folder_path(path, cx),
                        Some(_) => {}
                    }
                }
                view.picker_editor = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}
