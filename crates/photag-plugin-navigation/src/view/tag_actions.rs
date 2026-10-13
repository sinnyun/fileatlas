use super::*;

impl SidebarView {
    pub(super) fn create_tag(&mut self, cx: &mut Context<Self>) {
        let name = self.tag_name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        match self.service.create_tag(&name, self.parent_tag_id) {
            Ok(_) => {
                self.refresh();
                self.tag_create_open = false;
                self.parent_tag_id = None;
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn rename_tag(&mut self, id: i64, cx: &mut Context<Self>) {
        let name = self.tag_edit_name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        match self.service.rename_tag(id, &name) {
            Ok(()) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn update_tag_appearance(&mut self, id: i64, cx: &mut Context<Self>) {
        let name = self.tag_edit_name.read(cx).value().trim().to_owned();
        let color = self.tag_color.read(cx).value().trim().to_owned();
        let icon = self.tag_icon.read(cx).value().trim().to_owned();
        if !color.is_empty() && crate::parse_hex_color(&color).is_none() {
            self.snapshot = Err("Tag color must use #RRGGBB format".into());
            cx.notify();
            return;
        }
        if !name.is_empty() {
            if let Err(error) = self.service.rename_tag(id, &name) {
                self.snapshot = Err(error.to_string());
                cx.notify();
                return;
            }
        }
        if self.associated_edit_tag == Some(id) {
            let Some(tagging) = &self.tagging else {
                self.snapshot = Err("Tagging service is unavailable".into());
                cx.notify();
                return;
            };
            if let Err(error) = tagging.set_associations(id, &self.associated_tag_ids) {
                self.snapshot = Err(error.to_string());
                cx.notify();
                return;
            }
        }
        let color = if color.is_empty() { None } else { Some(color) };
        let icon = if icon.is_empty() { None } else { Some(icon) };
        let mut descendants = Vec::new();
        if self.apply_tag_appearance_to_children {
            if let Ok(snapshot) = &self.snapshot {
                if let Some(tag) = find_tag(&snapshot.tags, id) {
                    collect_tag_appearance_descendants(tag, &mut descendants);
                }
            }
        }
        match self
            .service
            .set_tag_appearance(id, color.as_deref(), icon.as_deref())
        {
            Ok(()) => {
                let mut propagation_error = None;
                for (child_id, has_explicit_appearance) in descendants {
                    if self.exclude_edited_tag_children && has_explicit_appearance {
                        continue;
                    }
                    if let Err(error) =
                        self.service
                            .set_tag_appearance(child_id, color.as_deref(), icon.as_deref())
                    {
                        propagation_error = Some(error.to_string());
                        break;
                    }
                }
                self.refresh();
                if let Some(error) = propagation_error {
                    self.snapshot = Err(error);
                }
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn begin_edit_tag(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.tag_edit_name_only = false;
        let Some(tagging) = &self.tagging else {
            self.snapshot = Err("Tagging service is unavailable".into());
            cx.notify();
            return;
        };
        match tagging.associated_with(id) {
            Ok(ids) => {
                self.associated_edit_tag = Some(id);
                self.associated_tag_ids = ids;
                if let Some(tag) = self
                    .snapshot
                    .as_ref()
                    .ok()
                    .and_then(|s| find_tag(&s.tags, id))
                {
                    let name = tag.name.clone();
                    let color = tag.color.clone().unwrap_or_default();
                    let icon = tag.icon.clone().unwrap_or_default();
                    self.tag_edit_display = tag.display;
                    self.tag_edit_name
                        .update(cx, |state, cx| state.set_value(&name, window, cx));
                    self.tag_color
                        .update(cx, |state, cx| state.set_value(&color, window, cx));
                    self.tag_icon
                        .update(cx, |state, cx| state.set_value(&icon, window, cx));
                }
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn begin_rename_tag(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tag) = self
            .snapshot
            .as_ref()
            .ok()
            .and_then(|s| find_tag(&s.tags, id))
        else {
            return;
        };
        let name = tag.name.clone();
        self.tag_edit_name
            .update(cx, |state, cx| state.set_value(&name, window, cx));
        self.tag_edit_name_only = true;
        self.associated_edit_tag = Some(id);
        cx.notify();
    }

    pub(super) fn toggle_associated_tag(&mut self, id: i64, cx: &mut Context<Self>) {
        if let Some(index) = self
            .associated_tag_ids
            .iter()
            .position(|tag_id| *tag_id == id)
        {
            self.associated_tag_ids.remove(index);
        } else {
            self.associated_tag_ids.push(id);
        }
        cx.notify();
    }

    pub(super) fn request_delete_tag(&mut self, id: i64, cx: &mut Context<Self>) {
        let tag = self
            .snapshot
            .as_ref()
            .ok()
            .and_then(|s| find_tag(&s.tags, id));
        let Some(tag) = tag else { return };
        let name = tag.name.clone();
        let count = tag.media_count.max(0) as usize;
        self.confirm_delete = Some((id, name, count));
        cx.notify();
    }

    pub(super) fn delete_tag(&mut self, id: i64, clean_parents: bool, cx: &mut Context<Self>) {
        match self.service.delete_tag(id, clean_parents) {
            Ok(affected) => {
                if let Some(tagging) = &self.tagging {
                    tagging.schedule_xmp_sync(&affected);
                }
                self.refresh();
                self.retain_existing_tag_filters();
                self.emit_tag_selection(cx);
                self.notify_media_index(cx);
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn begin_merge(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.merge_source = Some(id);
        self.merge_reparent_children = self
            .preferences
            .read()
            .map(|preferences| preferences.merge_reparent_children)
            .unwrap_or(true);
        self.merge_search
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    pub(super) fn merge_tag_into(&mut self, target: i64, cx: &mut Context<Self>) {
        let Some(source) = self.merge_source else {
            return;
        };
        let Some(tagging) = &self.tagging else {
            self.snapshot = Err("Tagging service is unavailable".into());
            cx.notify();
            return;
        };
        match tagging.merge(source, target, self.merge_reparent_children) {
            Ok(()) => {
                self.refresh();
                self.retain_existing_tag_filters();
                self.emit_tag_selection(cx);
                self.notify_media_index(cx);
                self.merge_source = None;
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn retain_existing_tag_filters(&mut self) {
        let mut live = HashSet::new();
        if let Ok(snapshot) = &self.snapshot {
            for tag in &snapshot.tags {
                collect_tag_ids(tag, &mut live);
            }
        }
        self.included_tags.retain(|id| live.contains(id));
        self.excluded_tags.retain(|id| live.contains(id));
    }

    pub(super) fn move_tag_into(&mut self, id: i64, cx: &mut Context<Self>) {
        let Some(target) = self.move_target_tag_id else {
            self.snapshot = Err(photag_ui_bridge::i18n::tr("Choose a target tag"));
            cx.notify();
            return;
        };
        match self.service.move_tag(
            id,
            photag_plugin_database::db::queries::TagDropPos::Into(target),
        ) {
            Ok(affected) => {
                self.refresh();
                self.move_target_tag_id = None;
                if !affected.is_empty() {
                    self.notify_media_index(cx);
                }
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn move_tag_to(
        &mut self,
        source: i64,
        drop: photag_plugin_database::db::queries::TagDropPos,
        cx: &mut Context<Self>,
    ) {
        let target = match drop {
            photag_plugin_database::db::queries::TagDropPos::Before(id)
            | photag_plugin_database::db::queries::TagDropPos::After(id)
            | photag_plugin_database::db::queries::TagDropPos::Into(id) => id,
        };
        if source == target {
            return;
        }
        match self.service.move_tag(source, drop) {
            Ok(affected) => {
                self.refresh();
                if !affected.is_empty() {
                    self.notify_media_index(cx);
                }
            }
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn toggle_tag_display(&mut self, id: i64, display: bool, cx: &mut Context<Self>) {
        match self.service.set_tag_display(id, display) {
            Ok(()) => self.refresh(),
            Err(error) => self.snapshot = Err(error.to_string()),
        }
        cx.notify();
    }

    pub(super) fn use_tag_as_parent(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.parent_tag_id = Some(id);
        self.tag_parent.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
    }
}
