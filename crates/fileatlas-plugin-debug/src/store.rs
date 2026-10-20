//! Bounded runtime diagnostics. No database or user-profile writes.
#[path = "selection.rs"]
mod selection;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, VecDeque},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

pub const CAPACITY: usize = 10_000;
pub const MAX_TEXT: usize = 4096;
#[derive(Clone, Debug, Serialize)]
pub struct Record {
    pub id: u64,
    pub time: String,
    pub elapsed_ms: f64,
    pub level: String,
    pub category: String,
    pub target: String,
    pub action: String,
    pub message: String,
    pub duration_ms: Option<f64>,
    pub span_id: Option<u64>,
    pub parent_span_id: Option<u64>,
    pub thread: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Filter {
    pub category: String,
    pub level: String,
    pub search: String,
    pub page: usize,
    pub page_size: usize,
    /// Freeze at a sequence number while inspecting older pages.
    pub through_id: Option<u64>,
    /// Hide routine DEBUG/TRACE accessibility repaint telemetry in the drawer.
    pub hide_ui_refresh: bool,
}
#[derive(Serialize)]
pub struct Page {
    pub records: Vec<Record>,
    pub total: usize,
    pub retained: usize,
    pub evicted: u64,
    pub page: usize,
    pub pages: usize,
    pub page_size: usize,
    pub latest_id: u64,
    pub categories: Vec<String>,
}
struct Buffer {
    records: VecDeque<Record>,
    next: u64,
    evicted: u64,
}
pub struct DebugService {
    start: Instant,
    capacity: usize,
    buffer: Mutex<Buffer>,
    revision: AtomicU64,
    content_revision: AtomicU64,
    ui_revision: AtomicU64,
    ui: Mutex<UiState>,
    pending_action: Mutex<Option<String>>,
}
impl cordis::Service for DebugService {
    const NAME: &'static str = "fileatlas/debug";
}
impl Default for DebugService {
    fn default() -> Self {
        Self::new(CAPACITY)
    }
}
impl DebugService {
    pub fn new(capacity: usize) -> Self {
        Self {
            start: Instant::now(),
            capacity: capacity.clamp(1, CAPACITY),
            buffer: Mutex::new(Buffer {
                records: VecDeque::new(),
                next: 1,
                evicted: 0,
            }),
            revision: AtomicU64::new(0),
            content_revision: AtomicU64::new(0),
            ui_revision: AtomicU64::new(0),
            ui: Mutex::new(UiState::default()),
            pending_action: Mutex::new(None),
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }
    /// Collection and presentation have different clocks: renderer telemetry
    /// remains queryable, but must not cause another render that logs itself.
    pub fn view_revision(&self, panel: bool) -> (u64, u64) {
        let ui = self.ui_state();
        (
            self.ui_revision.load(Ordering::Relaxed),
            if panel && ui.open && ui.filter.through_id.is_none() {
                self.content_revision.load(Ordering::Relaxed)
            } else {
                0
            },
        )
    }
    pub fn record(
        &self,
        level: &str,
        target: &str,
        action: &str,
        message: &str,
        duration_ms: Option<f64>,
        span_id: Option<u64>,
        parent_span_id: Option<u64>,
    ) {
        let mut b = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        if b.records.len() == self.capacity {
            b.records.pop_front();
            b.evicted += 1;
        }
        let id = b.next;
        b.next += 1;
        b.records.push_back(Record {
            id,
            time: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            elapsed_ms: self.start.elapsed().as_secs_f64() * 1000.,
            level: level.into(),
            category: bounded(&category(target)),
            target: bounded(target),
            action: bounded(action),
            message: bounded(message),
            duration_ms,
            span_id,
            parent_span_id,
            thread: format!("{:?}", std::thread::current().id()),
        });
        self.revision.fetch_add(1, Ordering::Relaxed);
        // GPUI emits this observation after painting the debug drawer itself.
        // Retain it above, without feeding the drawer's invalidation clock.
        if !is_ui_refresh(level, target, message) {
            self.content_revision.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn record_by_id(&self, id: u64) -> Option<Record> {
        self.buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .records
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }
    pub fn clear(&self) {
        let mut b = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        b.records.clear();
        b.evicted = 0;
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.content_revision.fetch_add(1, Ordering::Relaxed);
    }
    pub fn query(&self, filter: &Filter) -> Page {
        let b = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let search = filter.search.to_lowercase();
        let matches: Vec<_> = b
            .records
            .iter()
            .rev()
            .filter(|r| {
                (!filter.hide_ui_refresh || !is_ui_refresh(&r.level, &r.target, &r.message))
                    && filter.through_id.is_none_or(|id| r.id <= id)
                    && (filter.category.is_empty() || r.category == filter.category)
                    && (filter.level.is_empty() || r.level.eq_ignore_ascii_case(&filter.level))
                    && (search.is_empty()
                        || format!("{} {} {} {}", r.message, r.action, r.target, r.id)
                            .to_lowercase()
                            .contains(&search))
            })
            .collect();
        let page_size = if filter.page_size == 0 {
            50
        } else {
            filter.page_size.clamp(1, 200)
        };
        let pages = matches.len().div_ceil(page_size).max(1);
        let page = filter.page.min(pages - 1);
        Page {
            records: matches
                .iter()
                .skip(page * page_size)
                .take(page_size)
                .map(|r| (*r).clone())
                .collect(),
            total: matches.len(),
            retained: b.records.len(),
            evicted: b.evicted,
            page,
            pages,
            page_size,
            latest_id: b.next - 1,
            categories: b
                .records
                .iter()
                .filter(|r| {
                    !filter.hide_ui_refresh || !is_ui_refresh(&r.level, &r.target, &r.message)
                })
                .map(|r| r.category.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    }
}
fn is_ui_refresh(level: &str, target: &str, message: &str) -> bool {
    matches!(level, "DEBUG" | "TRACE")
        && target == "gpui::window"
        && message.contains("Sending a11y tree update:")
}

pub(crate) fn bounded(text: &str) -> String {
    if text.len() <= MAX_TEXT {
        return text.into();
    }
    let mut end = MAX_TEXT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [truncated]", &text[..end])
}
fn category(target: &str) -> String {
    target
        .split("::")
        .next()
        .unwrap_or(target)
        .strip_prefix("fileatlas_plugin_")
        .or_else(|| {
            target
                .split("::")
                .next()
                .unwrap_or(target)
                .strip_prefix("fileatlas_")
        })
        .unwrap_or_else(|| target.split("::").next().unwrap_or(target))
        .to_owned()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_filters_pages_clear_and_sequence() {
        let s = DebugService::new(3);
        for i in 0..5 {
            s.record(
                if i % 2 == 0 { "ERROR" } else { "INFO" },
                "fileatlas_plugin_database::query",
                "read",
                &format!("内容 {i}"),
                Some(1.5),
                None,
                None,
            );
        }
        let p = s.query(&Filter {
            page_size: 2,
            ..Default::default()
        });
        assert_eq!((p.total, p.retained, p.evicted, p.pages), (3, 3, 2, 2));
        assert_eq!(p.records[0].id, 5);
        let p = s.query(&Filter {
            page: 999,
            page_size: 2,
            ..Default::default()
        });
        assert_eq!(p.page, 1);
        assert_eq!(p.records[0].id, 3);
        let p = s.query(&Filter {
            category: "database".into(),
            level: "error".into(),
            search: "内容 4".into(),
            ..Default::default()
        });
        assert_eq!(p.total, 1);
        assert_eq!(
            s.query(&Filter {
                through_id: Some(3),
                ..Default::default()
            })
            .total,
            1
        );
        s.clear();
        assert_eq!(s.query(&Filter::default()).total, 0);
        s.record("INFO", "test", "done", "", None, None, None);
        assert_eq!(s.query(&Filter::default()).records[0].id, 6);
        assert!(bounded(&"汉".repeat(5000)).len() < MAX_TEXT + 32);
    }
    #[test]
    fn concurrent_writers_are_ordered_and_bounded() {
        let s = std::sync::Arc::new(DebugService::new(100));
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let s = s.clone();
                scope.spawn(move || {
                    for _ in 0..100 {
                        s.record("INFO", "test", "event", "", None, None, None);
                    }
                });
            }
        });
        let p = s.query(&Filter {
            page_size: 200,
            ..Default::default()
        });
        assert_eq!(p.retained, 100);
        assert_eq!(p.evicted, 300);
        assert!(p.records.windows(2).all(|p| p[0].id > p[1].id));
    }
}

#[derive(Clone, Serialize)]
pub struct UiState {
    pub open: bool,
    pub filter: Filter,
    pub selected: Option<u64>,
    pub selected_ids: Vec<u64>,
    pub anchor: Option<u64>,
    pub feedback: Option<String>,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            open: false,
            selected: None,
            selected_ids: Vec::new(),
            anchor: None,
            feedback: None,
            filter: Filter {
                hide_ui_refresh: true,
                ..Default::default()
            },
        }
    }
}
impl DebugService {
    pub fn ui_state(&self) -> UiState {
        self.ui.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn control(&self, action: &str, value: &str) -> Result<(), String> {
        match action {
            "select" => {
                return self.set_selection(
                    &[value.parse().map_err(|_| "Invalid record ID")?],
                    Some(value.parse().map_err(|_| "Invalid record ID")?),
                );
            }
            "select_ids" => {
                return self.set_selection(
                    &value
                        .split(',')
                        .filter(|v| !v.is_empty())
                        .map(|v| v.parse().map_err(|_| "Invalid record ID"))
                        .collect::<Result<Vec<u64>, _>>()?,
                    None,
                );
            }
            "select_range" => {
                let (start, end) = value.split_once(':').ok_or("Use start:end record IDs")?;
                return self.select_range(
                    start.parse().map_err(|_| "Invalid record ID")?,
                    end.parse().map_err(|_| "Invalid record ID")?,
                    false,
                );
            }
            "copy" | "export" => {
                *self
                    .pending_action
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(action.into());
                self.ui_revision.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            _ => {}
        }
        let mut ui = self.ui.lock().unwrap_or_else(|e| e.into_inner());
        match action {
            "toggle" => ui.open = !ui.open,
            "open" => ui.open = true,
            "close" => ui.open = false,
            "category" => {
                ui.filter.category = value.into();
                ui.filter.page = 0;
            }
            "level" => {
                if !["", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"].contains(&value) {
                    return Err("Invalid level".into());
                }
                ui.filter.level = value.into();
                ui.filter.page = 0;
            }
            "search" => {
                ui.filter.search = bounded(value);
                ui.filter.page = 0;
            }
            "page" => {
                ui.filter.page = value.parse().map_err(|_| "Invalid page")?;
                if ui.filter.page > 0 && ui.filter.through_id.is_none() {
                    ui.filter.through_id = Some(self.query(&Filter::default()).latest_id);
                }
            }
            "page_size" => {
                let n: usize = value.parse().map_err(|_| "Invalid page size")?;
                if ![25, 50, 100, 200].contains(&n) {
                    return Err("Invalid page size".into());
                }
                ui.filter.page_size = n;
                ui.filter.page = 0;
            }
            "pause" => ui.filter.through_id = Some(self.query(&Filter::default()).latest_id),
            "follow" => {
                ui.filter.through_id = None;
                ui.filter.page = 0;
            }
            "clear" => {
                self.clear();
                ui.filter.through_id = None;
                ui.filter.page = 0;
                ui.selected = None;
                ui.selected_ids.clear();
                ui.anchor = None;
                ui.feedback = None;
            }
            _ => return Err("Unknown debug action".into()),
        }
        if matches!(action, "category" | "level" | "search") {
            ui.selected = None;
            ui.selected_ids.clear();
            ui.anchor = None;
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.ui_revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    #[test]
    fn rendering_observations_are_retained_without_driving_more_renders() {
        let s = DebugService::new(100);
        s.control("open", "").unwrap();
        let drawer = s.view_revision(true);
        let button = s.view_revision(false);
        let collection = s.revision();
        for _ in 0..10 {
            s.record(
                "DEBUG",
                "gpui::window",
                "event",
                "message=Sending a11y tree update: 65 nodes",
                None,
                None,
                None,
            );
        }
        assert_eq!(s.query(&Filter::default()).total, 10);
        assert!(s.revision() > collection);
        assert_eq!(s.view_revision(true), drawer);
        assert_eq!(s.view_revision(false), button);
        s.record(
            "ERROR",
            "gpui::window",
            "event",
            "render failed",
            None,
            None,
            None,
        );
        assert_ne!(s.view_revision(true), drawer);
        assert_eq!(s.view_revision(false), button);
        s.control("pause", "").unwrap();
        let paused = s.view_revision(true);
        s.record(
            "INFO",
            "fileatlas_plugin_preview",
            "event",
            "decoded",
            None,
            None,
            None,
        );
        assert_eq!(s.view_revision(true), paused);
        s.control("close", "").unwrap();
        let hidden = s.view_revision(true);
        s.record(
            "INFO",
            "fileatlas_plugin_database",
            "read",
            "completed",
            None,
            None,
            None,
        );
        assert_eq!(s.view_revision(true), hidden);
    }
    #[test]
    fn drawer_hides_only_repaint_telemetry_before_filtering_and_paging() {
        let s = DebugService::new(100);
        for _ in 0..12 {
            s.record(
                "DEBUG",
                "gpui::window",
                "event",
                "Sending a11y tree update: 65 nodes",
                None,
                None,
                None,
            );
        }
        s.record(
            "WARN",
            "gpui::window",
            "event",
            "Sending a11y tree update: failed",
            None,
            None,
            None,
        );
        s.record(
            "ERROR",
            "gpui::window",
            "event",
            "render failed",
            None,
            None,
            None,
        );
        s.record(
            "DEBUG",
            "fileatlas_plugin_database",
            "read",
            "Sending a11y tree update: caller text",
            Some(1.0),
            None,
            None,
        );
        s.record(
            "INFO",
            "fileatlas_plugin_file_manager",
            "select",
            "selected",
            None,
            None,
            None,
        );
        let mut filter = s.ui_state().filter;
        assert!(filter.hide_ui_refresh);
        filter.page_size = 2;
        let first = s.query(&filter);
        assert_eq!(
            (
                first.total,
                first.pages,
                first.records.len(),
                first.retained
            ),
            (4, 2, 2, 16)
        );
        filter.page = 1;
        assert!(
            s.query(&filter)
                .records
                .iter()
                .all(|r| matches!(r.level.as_str(), "WARN" | "ERROR"))
        );
        filter.search = "65 nodes".into();
        assert_eq!(s.query(&filter).total, 0);
        assert_eq!(s.query(&Filter::default()).total, 16);
        assert!(s.record_by_id(1).is_some());
        s.clear();
        s.record(
            "TRACE",
            "gpui::window",
            "event",
            "Sending a11y tree update: 4 nodes",
            None,
            None,
            None,
        );
        assert!(s.query(&s.ui_state().filter).categories.is_empty());
        assert_eq!(s.query(&Filter::default()).categories, vec!["gpui"]);
    }

    #[test]
    fn historical_pages_freeze_view_without_stopping_collection() {
        let s = DebugService::new(100);
        s.record("INFO", "test", "one", "one", None, None, None);
        s.control("pause", "").unwrap();
        s.record("ERROR", "test", "two", "two", None, None, None);
        assert_eq!(s.query(&s.ui_state().filter).total, 1);
        s.control("follow", "").unwrap();
        assert_eq!(s.query(&s.ui_state().filter).total, 2);
        s.control("page", "1").unwrap();
        assert_eq!(s.ui_state().filter.through_id, Some(2));
        assert!(s.control("level", "bogus").is_err());
        assert!(s.control("page_size", "0").is_err());
        s.control("clear", "").unwrap();
        assert_eq!(s.ui_state().filter.page, 0);
        assert!(s.ui_state().filter.through_id.is_none());
        assert_eq!(s.query(&Filter::default()).retained, 0);
    }
}
