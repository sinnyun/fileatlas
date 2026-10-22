use super::*;

/// Titles belong to the native page order. The printed material number stays numeric.
pub fn page_title(index: usize) -> String {
    format!("页面{:02}", index + 1)
}

fn validate_order(boards: &[WorkCrateBoard], order: &[String]) -> Result<(), BoardsError> {
    let actual = boards
        .iter()
        .map(|b| b.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let requested = order
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    if order.len() != boards.len() || requested.len() != order.len() || requested != actual {
        return Err(BoardsError::Invalid(
            "页面列表已变化或排序包含重复页面，请同步后重试".into(),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PageOrderSave {
    pub boards: Vec<WorkCrateBoard>,
    pub changed: Vec<String>,
    pub frame: Option<CanvasSnapshot>,
}

pub(crate) fn apply_native_order(
    c: &mut impl NativeProtocol,
    project: &str,
    binding: &str,
    order: &[String],
) -> Result<(Vec<WorkCrateBoard>, Vec<String>), BoardsError> {
    let boards = native::catalog(c, project, binding)?;
    validate_order(&boards, order)?;
    let prior_positions = boards
        .iter()
        .map(|b| (b.id.clone(), b.index))
        .collect::<BTreeMap<_, _>>();
    c.execute(
        "edit.undoGroup.begin",
        json!({"label":"WorkCrate page order"}),
    )?;
    let result = (|| {
        let mut current = boards.iter().map(|b| b.id.clone()).collect::<Vec<_>>();
        for (to, id) in order.iter().enumerate() {
            let index = current
                .iter()
                .position(|value| value == id)
                .ok_or_else(|| BoardsError::BoardNotFound(id.clone()))?;
            if index != to {
                c.execute("artboard.reorder", json!({"index":index,"to":to}))?;
                let moved = current.remove(index);
                current.insert(to, moved);
            }
        }
        let mut boards = native::catalog(c, project, binding)?;
        let mut changed = Vec::new();
        let mut layout = native::owned_layout(c)?;
        for board in &mut boards {
            let title = page_title(board.index);
            // Unmoved pages have no new number or name: keep their nodes and PNGs.
            if prior_positions.get(&board.id) == Some(&board.index) && board.title == title {
                continue;
            }
            let report = header::write_in_group(c, board, &title, &board.info, &layout)?;
            if report.changed() {
                changed.push(board.id.clone());
            }
            if report.installed() {
                layout = native::owned_layout(c)?;
            }
            board.title = title;
        }
        Ok::<_, BoardsError>((boards, changed))
    })();
    match result {
        Ok(result) => {
            c.execute("edit.undoGroup.end", json!({}))?;
            Ok(result)
        }
        Err(error) => {
            c.execute("edit.undoGroup.end", json!({"cancel":true}))?;
            Err(error)
        }
    }
}

impl WorkCrateBoardsService {
    /// Metadata/order patches reuse the held document session and leave its canvas alive.
    pub fn reorder_pages(
        &self,
        project: &str,
        order: &[String],
    ) -> Result<PageOrderSave, BoardsError> {
        let _g = self.gate.lock().map_err(|_| BoardsError::Poisoned)?;
        validate_order(&self.board_catalog(project)?, order)?;
        let path = self
            .document_path(project)?
            .ok_or(BoardsError::NoDocument)?;
        let binding = self.projects.document_binding(project)?;
        if let Some(saved) = self.canvas_reorder_pages(project, order, &binding)? {
            return Ok(saved);
        }
        let mut c =
            self.official_session(std::slice::from_ref(&path), std::slice::from_ref(&path))?;
        c.open_document(&path)?;
        let (boards, changed) = apply_native_order(&mut c, project, &binding, order)?;
        c.save_file(&path)?;
        self.store_order(&mut c, project, &binding, boards, changed, None)
    }

    pub(crate) fn store_order(
        &self,
        c: &mut mcp::McpSession,
        project: &str,
        binding: &str,
        mut boards: Vec<WorkCrateBoard>,
        changed: Vec<String>,
        frame: Option<CanvasSnapshot>,
    ) -> Result<PageOrderSave, BoardsError> {
        self.merge_panel_fields(project, &mut boards);
        let mut cache = self
            .read_cache(project)?
            .ok_or_else(|| BoardsError::Invalid("页面缓存不可用，请先同步工程".into()))?;
        let changed_boards = boards
            .iter()
            .filter(|b| {
                changed.contains(&b.id) && frame.as_ref().is_none_or(|f| f.board_id != b.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        cache
            .previews
            .extend(self.make_previews(c, &changed_boards)?);
        if let Some(frame) = &frame
            && changed.contains(&frame.board_id)
            && let Some(source) = &frame.preview
        {
            let relative = format!("native-previews/{}.png", unique_token());
            std::fs::copy(
                source,
                self.projects.project_directory(project)?.join(&relative),
            )?;
            cache
                .previews
                .insert(format!("{}@1600", frame.board_id), relative);
        }
        cache.binding = binding.into();
        cache.boards = boards.clone();
        cache.source = self.source_stamp(project);
        self.write_cache(project, &cache)?;
        Ok(PageOrderSave {
            boards,
            changed,
            frame,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn titles_follow_the_current_position_with_at_least_two_digits() {
        assert_eq!(page_title(0), "页面01");
        assert_eq!(page_title(9), "页面10");
        assert_eq!(page_title(99), "页面100");
    }
    #[test]
    fn only_a_complete_permutation_is_accepted() {
        let boards = ["a", "b", "c"].map(|id| WorkCrateBoard {
            id: id.into(),
            project_id: "p".into(),
            native_id: 1,
            index: 0,
            title: String::new(),
            width_pt: 100.,
            height_pt: 100.,
            info: PageInfo::default(),
        });
        let ids = |values: &[&str]| values.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(validate_order(&boards, &ids(&["c", "a", "b"])).is_ok());
        for invalid in [
            vec!["a", "b"],
            vec!["a", "a", "c"],
            vec!["a", "b", "foreign"],
        ] {
            assert!(validate_order(&boards, &ids(&invalid)).is_err());
        }
    }
}
