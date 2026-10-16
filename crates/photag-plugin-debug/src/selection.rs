//! Stable multi-row selection and complete text export of retained diagnostics.
use super::*;

impl DebugService {
    pub fn freeze_view(&self, latest_id: u64) {
        let mut ui = self.ui.lock().unwrap_or_else(|e| e.into_inner());
        if ui.filter.through_id.is_none() {
            ui.filter.through_id = Some(latest_id);
            self.ui_revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn set_selection(&self, ids: &[u64], anchor: Option<u64>) -> Result<(), String> {
        if ids.len() > CAPACITY {
            return Err("Too many selected logs".into());
        }
        let ids = ids.iter().copied().collect::<BTreeSet<_>>();
        {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            if ids
                .iter()
                .any(|id| !buffer.records.iter().any(|r| r.id == *id))
            {
                return Err("Selected logs are no longer retained. Select them again.".into());
            }
        }
        let selected_ids = ids.into_iter().rev().collect::<Vec<_>>();
        let mut ui = self.ui.lock().unwrap_or_else(|e| e.into_inner());
        let selected = anchor
            .filter(|id| selected_ids.contains(id))
            .or_else(|| selected_ids.first().copied());
        if ui.selected_ids != selected_ids
            || ui.selected != selected
            || anchor.is_some_and(|id| ui.anchor != Some(id))
        {
            ui.selected_ids = selected_ids;
            ui.selected = selected;
            if anchor.is_some() {
                ui.anchor = anchor;
            }
            ui.feedback = None;
            self.ui_revision.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn select_range(&self, start: u64, end: u64, add: bool) -> Result<(), String> {
        let ui = self.ui_state();
        let page = self.query(&ui.filter);
        let ids = page.records.iter().map(|r| r.id).collect::<Vec<_>>();
        let a = ids
            .iter()
            .position(|id| *id == start)
            .ok_or("Selection anchor is outside this page")?;
        let b = ids
            .iter()
            .position(|id| *id == end)
            .ok_or("Selection target is outside this page")?;
        let mut selected = if add { ui.selected_ids } else { Vec::new() };
        selected.extend_from_slice(&ids[a.min(b)..=a.max(b)]);
        self.freeze_view(page.latest_id);
        self.set_selection(&selected, Some(start))
    }

    pub fn selected_text(&self) -> Result<String, String> {
        let ids = self.ui_state().selected_ids;
        if ids.is_empty() {
            return Err("Select logs first.".into());
        }
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let mut output = String::new();
        for id in ids {
            let record = buffer
                .records
                .iter()
                .find(|r| r.id == id)
                .ok_or("Selected logs are no longer retained. Select them again.")?;
            use std::fmt::Write;
            let duration = record
                .duration_ms
                .map(|v| format!("{v:.3} ms"))
                .unwrap_or_else(|| "—".into());
            let _ = writeln!(
                output,
                "{} [{}] [{}] #{} {} duration={} elapsed={:.3} ms",
                record.time,
                record.level,
                record.category,
                record.id,
                record.action,
                duration,
                record.elapsed_ms
            );
            let _ = writeln!(
                output,
                "target={} thread={} span={:?} parent={:?}\n{}\n",
                record.target, record.thread, record.span_id, record.parent_span_id, record.message
            );
        }
        Ok(output.replace("\r\n", "\n").replace('\n', "\r\n"))
    }

    pub fn export_selected(&self, path: &std::path::Path) -> Result<(), String> {
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
        {
            return Err("Choose a .txt file for the log export.".into());
        }
        std::fs::write(path, self.selected_text()?).map_err(|e| e.to_string())
    }

    pub fn feedback(&self, message: String) {
        let mut ui = self.ui.lock().unwrap_or_else(|e| e.into_inner());
        if ui.feedback.as_ref() != Some(&message) {
            ui.feedback = Some(message);
            self.ui_revision.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn take_action(&self) -> Option<String> {
        self.pending_action
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_freezes_order_exports_full_unicode_and_survives_paging() {
        let s = DebugService::new(100);
        for n in 1..=30 {
            s.record(
                "INFO",
                "photag_plugin_quicklook",
                "open",
                &format!("原始文件 & $ {n}\nsecond line"),
                Some(1.25),
                Some(5),
                None,
            );
        }
        s.control("page_size", "25").unwrap();
        s.select_range(29, 27, false).unwrap();
        assert_eq!(s.ui_state().selected_ids, vec![29, 28, 27]);
        let text = s.selected_text().unwrap();
        assert!(text.contains("原始文件 & $ 28\r\nsecond line"));
        assert!(text.contains("duration=1.250 ms"));
        assert!(text.find("#29").unwrap() < text.find("#27").unwrap());
        s.record("ERROR", "test", "later", "new", None, None, None);
        assert_eq!(s.query(&s.ui_state().filter).records[0].id, 30);
        s.control("page", "1").unwrap();
        assert_eq!(s.selected_text().unwrap(), text);
        let revision = s.view_revision(true);
        s.set_selection(&[29, 28, 27], Some(29)).unwrap();
        assert_eq!(s.view_revision(true), revision);
    }
    #[test]
    fn evicted_selection_is_reported_instead_of_silently_exporting_partial_text() {
        let s = DebugService::new(2);
        s.record("INFO", "test", "one", "one", None, None, None);
        s.control("select", "1").unwrap();
        s.record("INFO", "test", "two", "two", None, None, None);
        s.record("INFO", "test", "three", "three", None, None, None);
        assert!(
            s.selected_text()
                .unwrap_err()
                .contains("no longer retained")
        );
        s.control("clear", "").unwrap();
        assert!(s.ui_state().selected_ids.is_empty());
    }

    #[test]
    fn export_writes_the_same_complete_crlf_text_and_requires_txt_extension() {
        let s = DebugService::new(10);
        s.record(
            "ERROR",
            "photag_plugin_quicklook",
            "open",
            "原图 & $\nsecond line",
            Some(2.5),
            None,
            None,
        );
        s.control("select", "1").unwrap();
        let path =
            std::env::temp_dir().join(format!("photag-debug-export-{}.txt", std::process::id()));
        s.export_selected(&path).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            s.selected_text().unwrap()
        );
        assert!(s.export_selected(&path.with_extension("log")).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
