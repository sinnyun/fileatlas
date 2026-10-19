//! Unified favorite shortcuts; library indexing/registration remains independent.
use super::*;

impl SidebarView {
    pub(super) fn render_favorite_folder_creation(&self, cx: &mut Context<Self>) -> Option<Div> {
        let parent = self.new_folder_parent.as_ref()?;
        Some(
            div()
                .v_flex()
                .gap_1()
                .p_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme::text_secondary()))
                        .child(parent.clone()),
                )
                .child(photag_ui_bridge::components::text_input(&self.new_folder_name).w_full())
                .child(
                    div()
                        .h_flex()
                        .gap_1()
                        .child(
                            Button::new("navigation-create-subfolder")
                                .label(photag_ui_bridge::i18n::tr("Create"))
                                .on_click(cx.listener(|this, _, _, cx| this.create_subfolder(cx))),
                        )
                        .child(
                            Button::new("navigation-cancel-subfolder")
                                .label(photag_ui_bridge::i18n::tr("Cancel"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.new_folder_parent = None;
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }
    pub(super) fn import_library_favorites(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.favorite_import_pending {
            return false;
        }
        self.favorite_import_pending = false;
        let Ok(snapshot) = &self.snapshot else {
            return false;
        };
        let roots = snapshot
            .folders
            .iter()
            .map(|node| {
                (
                    node.folder.id,
                    node.folder.path.clone(),
                    node.folder.name.clone(),
                )
            })
            .collect::<Vec<_>>();
        let Some(config) = self.folders.import_library_folders(
            &roots,
            &photag_ui_bridge::i18n::tr("Default favorite group"),
        ) else {
            return false;
        };
        self.sidebar_config_revision = self.folders.revision();
        self.dispatch_sidebar_save(config, cx);
        true
    }

    /// Read the committed location, never unsubmitted address/search input.
    pub(super) fn current_favorite_path(&self) -> Option<String> {
        let saved = self.session.snapshot().ok()?;
        match address_bar::location_from_session(&saved, self.snapshot.as_ref().ok()) {
            crate::location::Location::Directory(path) => Some(path),
            _ => None,
        }
    }

    pub(super) fn add_current_favorite(&mut self, group_id: String, cx: &mut Context<Self>) {
        let Some(path) = self.current_favorite_path() else {
            return;
        };
        self.add_favorite_folder_path(group_id.clone(), path, cx);
        if self.collapsed_groups.remove(&group_id) {
            self.persist_sidebar_expansion();
        }
        cx.notify();
    }
}
