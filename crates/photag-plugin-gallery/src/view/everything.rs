//! Inline dependency feedback. Passive registry/file reads stay off render.
use super::*;
impl GalleryView {
    pub(super) fn set_search_entire_computer(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.filter.search_entire_computer == enabled {
            return;
        }
        if let Some(settings) = &self.settings {
            if let Err(error) =
                settings.update(|settings| settings.search_entire_computer = enabled)
            {
                self.operation_status = Some(format!("Couldn't save search location: {error}"));
                cx.notify();
                return;
            }
        }
        self.filter.search_entire_computer = enabled;
        self.reload();
        cx.notify();
    }

    pub(super) fn render_search_location(&self, cx: &mut Context<Self>) -> AnyElement {
        let entire = self.filter.search_entire_computer;
        let (label, tooltip) = if entire {
            (
                "Entire computer",
                "Search all locations indexed by Everything",
            )
        } else {
            (
                "Current folder",
                if self.filter.path_prefix.is_none() && self.filter.folder_id.is_none() {
                    "No folder selected: search registered folders and their subfolders"
                } else {
                    "Search the current folder and all subfolders"
                },
            )
        };
        view_controls::button(
            "gallery-search-location",
            if entire {
                gpui_kit::assets::IconName::Globe
            } else {
                gpui_kit::assets::IconName::FolderSearch
            },
            label,
        )
        .toggled(entire)
        .tooltip(photag_ui_bridge::i18n::tr(tooltip))
        .on_click(cx.listener(move |view, _, _, cx| view.set_search_entire_computer(!entire, cx)))
        .into_any_element()
    }

    pub(super) fn recheck_everything(&mut self, cx: &mut Context<Self>) {
        self.everything_check_revision = self.everything_check_revision.wrapping_add(1);
        if !self.filter.search.trim().is_empty() {
            self.reload();
        }
        cx.notify();
    }
    pub(super) fn poll_everything_installation(&mut self) -> bool {
        let Some(service) = self.service.everything.clone() else {
            return false;
        };
        self.everything_installation
            .update(self.everything_check_revision, move || {
                Ok(service.inspect_installation())
            })
    }

    pub(super) fn everything_status(
        &self,
    ) -> Option<&photag_plugin_everything_search::EverythingInstallation> {
        self.everything_installation
            .value()
            .and_then(|value| value.as_ref().ok())
    }
}
