#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Text clipboard history workbench. Uses clipboard-master (MIT) for OS change
//! notifications and arboard (MIT/Apache-2.0) for clipboard text access.
use clipboard_master::{CallbackResult, ClipboardHandler, Master, Shutdown};
use cordis::{Context, Plugin, Service};
use photag_plugin_database::{ClipboardCategoryRecord, ClipboardHistoryRecord, DatabaseService};
use photag_ui_bridge::{
    ContributionKind, UiBridgeService, UiContribution, register_for_plugin,
    register_views_for_plugin,
};
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

const OWNER: &str = "photag.clipboard";
const PANEL: &str = "clipboard-workbench/panel";
const OPEN: &str = "clipboard-workbench/open";
pub mod content;
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

pub struct ClipboardService {
    db: Arc<DatabaseService>,
    items: Mutex<Vec<ClipboardHistoryRecord>>,
    categories: Mutex<Vec<ClipboardCategoryRecord>>,
    shutdown: Mutex<Option<Shutdown>>,
    stopped: AtomicBool,
    paused: AtomicBool,
    revision: AtomicU64,
    wake: photag_ui_bridge::refresh::Wake,
}
impl Service for ClipboardService {
    const NAME: &'static str = "photag/clipboard";
}
impl ClipboardService {
    fn changed(&self) -> Result<(), String> {
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn assign_category(&self, id: i64, category: Option<i64>) -> Result<(), String> {
        if let Some(category) = category {
            if !self.categories()?.iter().any(|item| item.id == category) {
                return Err("分组已不存在，请重新选择".into());
            }
        }
        self.db
            .clipboard_assign_category(id, category)
            .map_err(|e| e.to_string())?;
        self.changed()
    }
    pub fn rename_category(&self, id: i64, name: &str) -> Result<(), String> {
        self.validate_group_name(name, Some(id))?;
        self.db
            .clipboard_rename_category(id, name.trim())
            .map_err(|e| e.to_string())?;
        self.changed()
    }
    pub fn delete_category(&self, id: i64) -> Result<(), String> {
        self.db
            .clipboard_delete_category(id)
            .map_err(|e| e.to_string())?;
        self.changed()
    }
    fn capture(&self) -> Result<(), String> {
        if self.paused.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        let text = clipboard.get_text().map_err(|e| e.to_string())?;
        if text.trim().is_empty() || text.len() > 1_000_000 {
            return Ok(());
        }
        self.db
            .clipboard_capture(&text, now(), 500)
            .map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Vec<ClipboardHistoryRecord>, String> {
        self.items
            .lock()
            .map(|v| v.clone())
            .map_err(|_| "clipboard cache unavailable".into())
    }
    pub fn categories(&self) -> Result<Vec<ClipboardCategoryRecord>, String> {
        self.categories
            .lock()
            .map(|v| v.clone())
            .map_err(|_| "clipboard categories unavailable".into())
    }
    fn reload(&self) -> Result<(), String> {
        let (categories, items) = self.db.clipboard_snapshot().map_err(|e| e.to_string())?;
        *self
            .categories
            .lock()
            .map_err(|_| "clipboard categories unavailable")? = categories;
        *self
            .items
            .lock()
            .map_err(|_| "clipboard cache unavailable")? = items;
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    pub fn wake(&self) -> photag_ui_bridge::refresh::Wake {
        self.wake.clone()
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
    }
    pub fn use_item(&self, id: i64) -> Result<(), String> {
        let text = self
            .db
            .clipboard_mark_used(id, now())
            .map_err(|e| e.to_string())?;
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(text).map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn save_item(
        &self,
        id: i64,
        content: &str,
        category: Option<i64>,
        pinned: bool,
    ) -> Result<(), String> {
        if content.trim().is_empty() {
            return Err("内容不能为空".into());
        }
        if content.len() > 1_000_000 {
            return Err("内容超过 1 MB".into());
        }
        self.db
            .clipboard_update(id, content, category, pinned, now())
            .map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn add_text(
        &self,
        content: &str,
        category: Option<i64>,
        pinned: bool,
    ) -> Result<i64, String> {
        if content.trim().is_empty() {
            return Err("内容不能为空".into());
        }
        if content.len() > 1_000_000 {
            return Err("内容超过 1 MB".into());
        }
        let id = self
            .db
            .clipboard_add_manual(content, category, pinned, now())
            .map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(id)
    }
    pub fn add_favorite(&self, id: i64) -> Result<(), String> {
        self.db
            .clipboard_add_favorite(id)
            .map_err(|e| e.to_string())?;
        self.changed()
    }
    pub fn remove_favorite(&self, id: i64) -> Result<(), String> {
        self.db
            .clipboard_remove_favorite(id)
            .map_err(|e| e.to_string())?;
        self.changed()
    }
    pub fn delete(&self, id: i64) -> Result<(), String> {
        self.db.clipboard_delete(id).map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn clear_unpinned(&self) -> Result<(), String> {
        self.db
            .clipboard_clear_unpinned()
            .map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn add_category(&self, name: &str) -> Result<(), String> {
        let name = name.trim();
        self.validate_group_name(name, None)?;
        self.db
            .clipboard_add_category(name)
            .map_err(|e| e.to_string())?;
        self.reload()?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    fn validate_group_name(&self, name: &str, current: Option<i64>) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("分组名称不能为空".into());
        }
        if name.chars().count() > 40 {
            return Err("分组名称不能超过 40 个字".into());
        }
        let categories = self.categories()?;
        if current.is_some_and(|id| !categories.iter().any(|group| group.id == id)) {
            return Err("分组已不存在".into());
        }
        if categories
            .iter()
            .any(|group| Some(group.id) != current && group.name == name)
        {
            return Err("已存在同名分组".into());
        }
        Ok(())
    }
    fn start(self: &Arc<Self>) {
        let service = Arc::clone(self);
        let weak = Arc::downgrade(self);
        let worker = std::thread::Builder::new()
            .name("photag-clipboard-listener".into())
            .spawn(move || {
                struct Handler(std::sync::Weak<ClipboardService>);
                impl ClipboardHandler for Handler {
                    fn on_clipboard_change(&mut self) -> CallbackResult {
                        if let Some(s) = self.0.upgrade() {
                            if let Err(e) = s.capture() {
                                tracing::debug!(%e,"clipboard text capture skipped");
                            }
                        }
                        CallbackResult::Next
                    }
                    fn on_clipboard_error(&mut self, e: io::Error) -> CallbackResult {
                        tracing::warn!(%e,"clipboard listener error");
                        CallbackResult::Next
                    }
                }
                match Master::new(Handler(weak)) {
                    Ok(mut m) => {
                        let shutdown = m.shutdown_channel();
                        if let Ok(mut slot) = service.shutdown.lock() {
                            if service.stopped.load(Ordering::Acquire) {
                                shutdown.signal();
                            } else {
                                *slot = Some(shutdown);
                            }
                        }
                        let _ = m.run();
                    }
                    Err(e) => tracing::error!(%e,"clipboard listener initialization failed"),
                }
            });
        if let Err(e) = worker {
            tracing::error!(%e,"clipboard listener thread failed");
        }
    }
}
#[derive(Clone, Copy)]
pub struct ClipboardPlugin;
impl Plugin for ClipboardPlugin {
    type Config = ();
    type Input = ();
    type PrepareError = std::convert::Infallible;
    type ApplyError = ClipboardPluginError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }
    fn prepare(&self, _: ()) -> Result<(), Self::PrepareError> {
        Ok(())
    }
    async fn apply(&self, context: Context, _: &()) -> Result<(), ClipboardPluginError> {
        let result: Result<(), String> = async {
            let db = context
                .try_service::<DatabaseService>()
                .map_err(|e| e.to_string())?;
            let (categories, items) = db.clipboard_snapshot().map_err(|e| e.to_string())?;
            let service = Arc::new(ClipboardService {
                db,
                items: Mutex::new(items),
                categories: Mutex::new(categories),
                shutdown: Mutex::new(None),
                stopped: AtomicBool::new(false),
                paused: AtomicBool::new(false),
                revision: AtomicU64::new(0),
                wake: Default::default(),
            });
            let _publication = context
                .provide(Arc::clone(&service))
                .map_err(|e| e.to_string())?;
            let cleanup = Arc::clone(&service);
            context
                .effect_sync(move || {
                    cleanup.stopped.store(true, Ordering::Release);
                    if let Ok(mut slot) = cleanup.shutdown.lock() {
                        if let Some(stop) = slot.take() {
                            stop.signal();
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
            let bridge = context
                .try_service::<UiBridgeService>()
                .map_err(|e| e.to_string())?;
            let panels = Arc::clone(&bridge.panels);
            let command_panels = Arc::clone(&panels);
            let _listener = context
                .on::<photag_ui_bridge::UiCommandInvoked, _>(cordis::event::observer_sync(
                    move |_, id| {
                        if id == OPEN {
                            let _ = command_panels.open(PANEL);
                        }
                        Ok::<_, std::convert::Infallible>(())
                    },
                ))
                .map_err(|e| e.to_string())?;
            register_for_plugin(
                &context,
                OWNER,
                [
                    UiContribution::new(
                        "clipboard-workbench/panel",
                        ContributionKind::Panel {
                            label: "剪贴板工作台".into(),
                            slot: PANEL.into(),
                            order: 0,
                        },
                    ),
                    UiContribution::new(
                        OPEN,
                        ContributionKind::Command {
                            label: "打开剪贴板工作台".into(),
                            needs: photag_ui_bridge::CommandNeeds::None,
                        },
                    ),
                    UiContribution::new(
                        "clipboard-workbench/open-key",
                        ContributionKind::Keybinding {
                            key: "Ctrl+Shift+V".into(),
                            command_id: OPEN.into(),
                            scope: photag_ui_bridge::ShortcutScope::System,
                        },
                    ),
                    UiContribution::new(
                        "clipboard-workbench/sidebar-button",
                        ContributionKind::Widget {
                            label: "Clipboard".into(),
                            slot: "navigation.launcher-shortcuts".into(),
                            order: 5,
                        },
                    ),
                ],
            )
            .map_err(|e| e.to_string())?;
            let panel_service = Arc::clone(&service);
            let surfaces = Arc::clone(&bridge.surfaces);
            photag_ui_bridge::panel_catalog::register_panel_for_plugin(
                &context,
                OWNER,
                photag_ui_bridge::panel_catalog::PanelDefinition {
                    options: photag_ui_bridge::panels::PanelOptions {
                        id: PANEL.into(),
                        title: "剪贴板工作台".into(),
                        default_size: (1160., 760.),
                        min_size: (1000., 660.),
                    },
                    multiplicity: photag_ui_bridge::panel_catalog::PanelMultiplicity::Single,
                    factory: Arc::new(move |window, app| {
                        view::panel(
                            Arc::clone(&panel_service),
                            Arc::clone(&surfaces),
                            window,
                            app,
                        )
                    }),
                },
            )
            .map_err(|e| e.to_string())?;
            register_views_for_plugin(&context, OWNER, [view::sidebar_button(panels)])
                .map_err(|e| e.to_string())?;
            service.start();
            Ok(())
        }
        .await;
        result.map_err(ClipboardPluginError)
    }
}
#[derive(Debug, thiserror::Error)]
#[error("clipboard plugin startup failed: {0}")]
pub struct ClipboardPluginError(String);
mod view;
