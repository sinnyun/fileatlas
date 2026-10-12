//! Individual and bulk filter reset actions.

use gpui_kit::*;

use super::GalleryView;
use crate::filter::{GalleryFilter, MediaTypeFilter};

impl GalleryView {
    pub(super) fn clear_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let folder_id = self.filter.folder_id;
        let path_prefix = self.filter.path_prefix.clone();
        let selected_tag_ids = self.filter.selected_tag_ids.clone();
        self.filter = GalleryFilter {
            folder_id,
            path_prefix,
            selected_tag_ids,
            ..GalleryFilter::default()
        };
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.date_from
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.date_to
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.reload();
        cx.notify();
    }

    pub(super) fn clear_search_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter.search.clear();
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.reload();
        cx.notify();
    }

    pub(super) fn clear_date_filter(
        &mut self,
        from: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = if from { &self.date_from } else { &self.date_to };
        if from {
            self.filter.date_from = None;
        } else {
            self.filter.date_to = None;
        }
        input.update(cx, |input, cx| input.set_value("", window, cx));
        self.reload();
        cx.notify();
    }

    pub(super) fn clear_type_filter(&mut self, cx: &mut Context<Self>) {
        self.filter.media_type = MediaTypeFilter::All;
        self.reload();
        cx.notify();
    }

    pub(super) fn clear_rating_filter(&mut self, cx: &mut Context<Self>) {
        self.filter.rating_min = 0;
        self.filter.rating_max = 5;
        self.reload();
        cx.notify();
    }

    pub(super) fn clear_extension_filter(&mut self, extension: String, cx: &mut Context<Self>) {
        self.filter.clear_extension(&extension);
        self.reload();
        cx.notify();
    }
}
