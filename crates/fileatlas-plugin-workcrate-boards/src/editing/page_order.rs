use super::*;

impl WorkCrateBoardsService {
    /// The service gate is held; keep the selected native page, tool and official process.
    pub(crate) fn canvas_reorder_pages(
        &self,
        project: &str,
        order: &[String],
        binding: &str,
    ) -> Result<Option<PageOrderSave>, BoardsError> {
        let mut slot = self.canvas.lock().map_err(|_| BoardsError::Poisoned)?;
        let Some(state) = slot.as_mut().filter(|s| s.project_id == project) else {
            return Ok(None);
        };
        state.session.inspect_bound()?;
        let (boards, changed) =
            crate::page_order::apply_native_order(&mut state.session, project, binding, order)?;
        let board = boards
            .iter()
            .find(|b| b.native_id == state.native_id)
            .ok_or_else(|| BoardsError::BoardNotFound(state.board_id.clone()))?;
        // Like switch_to, update only our preview index. A viewport command is
        // unavailable headlessly and must not move the user's live GUI viewport.
        state.artboard = board.index;
        state.session.save_file(&state.document)?;
        let frame = state.snapshot_after(changed.contains(&state.board_id))?;
        self.store_order(
            &mut state.session,
            project,
            binding,
            boards,
            changed,
            Some(frame),
        )
        .map(Some)
    }
}
