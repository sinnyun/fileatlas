use super::*;
impl SidebarView {
    pub(super) fn dispatch_sidebar_save(
        &mut self,
        config: fileatlas_contracts::SidebarFoldersConfig,
        cx: &mut Context<Self>,
    ) {
        let revision = self.folders.revision();
        let cordis = self.cordis.clone();
        cx.spawn(async move |view, cx| {
            let result = cordis
                .emit::<fileatlas_contracts::SidebarFoldersChanged>(
                    Routing::Unscoped,
                    fileatlas_contracts::SidebarFoldersUpdate { revision, config },
                )
                .await;
            let _ = view.update(cx, |view, cx| {
                if view.folders.revision() == revision {
                    let error = result.err().map(|error| error.to_string());
                    if error != view.sidebar_save_error {
                        if let Some(error) = &error {
                            tracing::error!(%error,"sidebar preferences save failed");
                        }
                        view.sidebar_save_error = error;
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }
    pub(super) fn retry_sidebar_save(&mut self, cx: &mut Context<Self>) {
        self.dispatch_sidebar_save(self.sidebar_config(), cx);
    }
    pub(super) fn persist_sidebar_expansion(&self) {
        if let Ok(mut saved) = self.session.snapshot() {
            saved.sidebar_collapsed_sections = self
                .collapsed_sections
                .iter()
                .map(|s| s.to_string())
                .collect();
            saved.sidebar_collapsed_groups = self.collapsed_groups.clone();
            if let Err(error) = self.session.replace(saved) {
                tracing::error!(%error,"sidebar expansion save failed");
            }
        }
    }
    pub(super) fn toggle_favorite_group(&mut self, id: &str) {
        if !self.collapsed_groups.remove(id) {
            self.collapsed_groups.insert(id.to_owned());
        }
        self.persist_sidebar_expansion();
    }
}
