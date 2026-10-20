use super::*;

impl SidebarView {
    pub(super) fn select_folder(&mut self, folder_id: Option<i64>, cx: &mut Context<Self>) {
        if let Ok(mut session) = self.session.snapshot() {
            self.included_tags.clear();
            self.excluded_tags.clear();
            session.selected_tag_ids.clear();
            session.excluded_tag_ids.clear();
            session.selected_folder_id = folder_id;
            session.selected_path = None;
            if let Err(error) = self.session.replace(session) {
                tracing::error!("folder selection session save failed: {error}");
            }
        }
        self.sync_address_location();
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<FolderSelectionChanged>(Routing::Unscoped, folder_id)
                .await
            {
                tracing::error!("folder selection dispatch failed: {error}");
            }
        })
        .detach();
    }

    pub(super) fn emit_tag_selection(&mut self, cx: &mut Context<Self>) {
        if let Ok(mut saved) = self.session.snapshot() {
            saved.selected_tag_ids = self.included_tags.clone();
            saved.excluded_tag_ids = self.excluded_tags.clone();
            if !self.included_tags.is_empty() || !self.excluded_tags.is_empty() {
                saved.selected_path = None;
                saved.selected_folder_id = None;
            }
            if let Err(error) = self.session.replace(saved) {
                tracing::error!(%error,"tag navigation session save failed");
            }
        }
        self.sync_address_location();
        let cordis = self.cordis.clone();
        let selection = TagFilterSelection {
            included: self.included_tags.clone(),
            excluded: self.excluded_tags.clone(),
        };
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<TagFilterSelectionChanged>(Routing::Unscoped, selection)
                .await
            {
                tracing::error!("tag selection dispatch failed: {error}");
            }
        })
        .detach();
    }

    pub(super) fn select_path(&mut self, path: String, cx: &mut Context<Self>) {
        if let Ok(mut session) = self.session.snapshot() {
            self.included_tags.clear();
            self.excluded_tags.clear();
            session.selected_tag_ids.clear();
            session.excluded_tag_ids.clear();
            session.selected_folder_id = None;
            session.selected_path = Some(path.clone());
            if let Err(error) = self.session.replace(session) {
                tracing::error!("path selection session save failed: {error}");
            }
        }
        self.sync_address_location();
        let cordis = self.cordis.clone();
        cx.spawn(async move |_, _| {
            if let Err(error) = cordis
                .emit::<DirectoryPathSelected>(Routing::Unscoped, Some(path))
                .await
            {
                tracing::error!("directory selection dispatch failed: {error}");
            }
        })
        .detach();
    }
}
