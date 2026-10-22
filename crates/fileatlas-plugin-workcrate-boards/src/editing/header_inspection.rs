use super::*;

impl WorkCrateBoardsService {
    /// Explicit diagnostics on the already-open official session; opens no document or process.
    pub fn canvas_header_state(&self) -> Result<Value, BoardsError> {
        let mut snapshot = self.canvas_call(|state| {
            let inspect = state.session.inspect_bound()?;
            let layout = native::owned_layout(&mut state.session)?;
            let owned = layout.boards.get(&state.native_id).cloned().unwrap_or_default();
            let mut nodes = BTreeMap::new();
            for (role, node) in &owned.fields {
                let summary = state.session.execute("document.node", json!({"id":node.id,"summary":true}))?;
                let model = state.session.execute("document.node", json!({"id":node.id}))?;
                nodes.insert(role.clone(), json!({"id":node.id,"text":node.text,"note":node.note,"summary":summary,"model":model}));
            }
            let mut furniture = BTreeMap::new();
            for (role, id) in [("band",owned.band),("chip",owned.chip),("page_group",owned.page_group),("master",layout.header_root)] {
                if let Some(id) = id { furniture.insert(role, state.session.execute("document.node",json!({"id":id,"summary":true,"depth":0}))?); }
            }
            Ok(json!({"board_id":state.board_id,"artboards":inspect["artboards"],"layout":owned.layout_version,"nodes":nodes,"furniture":furniture}))
        })?;
        snapshot["last_write"] = self
            .header_report
            .lock()
            .map_err(|_| BoardsError::Poisoned)?
            .clone();
        let (project, _) = native::split_id(snapshot["board_id"].as_str().unwrap_or_default())?;
        snapshot["previews"] = serde_json::to_value(self.read_cache(project)?.map(|c| c.previews))?;
        Ok(snapshot)
    }
}
