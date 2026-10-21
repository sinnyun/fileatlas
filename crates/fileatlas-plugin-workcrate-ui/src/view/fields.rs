use super::*;
impl WorkCratePanel {
    pub(super) fn set_field_value(
        &mut self,
        id: String,
        key: String,
        value: String,
        cx: &mut ViewContext<Self>,
    ) {
        let boards = self.boards.clone();
        self.perform(
            move || {
                boards
                    .update_content_field(&id, &key, &value)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
            cx,
        );
    }
}
