use super::*;
impl WorkCratePanel {
    pub(super) fn set_field_value(
        &mut self,
        id: String,
        key: String,
        value: String,
        cx: &mut ViewContext<Self>,
    ) {
        self.editing = Some((id, key));
        self.save_value(value, cx);
    }
}
