#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Clipboard history workbench. JSON owns feature data; image assets are files.
//! clipboard-master (MIT) observes changes and arboard reads clipboard content.
use clipboard_master::{CallbackResult, ClipboardHandler, Master, Shutdown};
use cordis::{Context, Plugin, Service};
use fileatlas_plugin_database::{ClipboardCategoryRecord, ClipboardHistoryRecord, DatabaseService};
use fileatlas_plugin_settings::{PluginLaunchMode, SettingsService};
use fileatlas_ui_bridge::{
    ContributionKind, UiBridgeService, UiContribution, register_for_plugin,
    register_views_for_plugin,
};
use std::{
    borrow::Cow,
    io,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

const OWNER: &str = "fileatlas.clipboard";
const PANEL: &str = "clipboard-workbench/panel";
const OPEN: &str = "clipboard-workbench/open";
pub mod content;
mod mcp;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ClipboardFile {
    version: u32,
    categories: Vec<ClipboardCategoryRecord>,
    items: Vec<ClipboardHistoryRecord>,
    next_id: i64,
}
impl ClipboardFile {
    fn persist(&self, path: &std::path::Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let parent = path.parent().ok_or("剪贴板数据目录无效")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
        if let Err(first) = std::fs::rename(&temp, path) {
            if !path.exists() {
                return Err(first.to_string());
            }
            let backup = path.with_extension("json.bak");
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(path, &backup).map_err(|e| e.to_string())?;
            if let Err(error) = std::fs::rename(&temp, path) {
                let _ = std::fs::rename(&backup, path);
                return Err(error.to_string());
            }
            let _ = std::fs::remove_file(backup);
        }
        Ok(())
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

pub struct ClipboardService {
    data_path: PathBuf,
    data: Mutex<ClipboardFile>,
    items: Mutex<Vec<ClipboardHistoryRecord>>,
    categories: Mutex<Vec<ClipboardCategoryRecord>>,
    shutdown: Mutex<Option<Shutdown>>,
    stopped: AtomicBool,
    paused: AtomicBool,
    revision: AtomicU64,
    wake: fileatlas_ui_bridge::refresh::Wake,
}
impl Service for ClipboardService {
    const NAME: &'static str = "fileatlas/clipboard";
}
impl ClipboardService {
    fn update_file<R>(
        &self,
        update: impl FnOnce(&mut ClipboardFile) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut data = self.data.lock().map_err(|_| "clipboard data unavailable")?;
        let mut next = data.clone();
        let result = update(&mut next)?;
        next.items
            .sort_by_key(|item| (!item.pinned, std::cmp::Reverse(item.last_used_at)));
        next.persist(&self.data_path)?;
        *data = next;
        drop(data);
        self.reload()?;
        Ok(result)
    }
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
        self.update_file(|data| {
            let item = data
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("剪贴板记录已不存在")?;
            item.category_id = category;
            Ok(())
        })?;
        self.changed()
    }
    pub fn rename_category(&self, id: i64, name: &str) -> Result<(), String> {
        self.validate_group_name(name, Some(id))?;
        self.update_file(|data| {
            let category = data
                .categories
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("分组已不存在")?;
            category.name = name.trim().to_owned();
            Ok(())
        })?;
        self.changed()
    }
    pub fn delete_category(&self, id: i64) -> Result<(), String> {
        self.update_file(|data| {
            data.categories.retain(|item| item.id != id);
            for item in &mut data.items {
                if item.category_id == Some(id) {
                    item.category_id = None;
                }
            }
            Ok(())
        })?;
        self.changed()
    }
    fn capture(&self) -> Result<(), String> {
        if self.paused.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        if let Ok(text) = clipboard.get_text()
            && !text.trim().is_empty()
            && text.len() <= 1_000_000
        {
            self.update_file(|data| {
                capture_text(data, &text, now());
                Ok(())
            })?;
        }
        if let Ok(image) = clipboard.get_image() {
            const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;
            let Some(pixel_count) = image.width.checked_mul(image.height) else {
                return Err("剪贴板图片尺寸无效".into());
            };
            if image.bytes.len() <= MAX_IMAGE_BYTES && pixel_count > 0 {
                let image_dir = image_store_directory()?;
                std::fs::create_dir_all(&image_dir).map_err(|e| e.to_string())?;
                let mut digest = sha2::Sha256::new();
                use sha2::Digest as _;
                digest.update((image.width as u64).to_le_bytes());
                digest.update((image.height as u64).to_le_bytes());
                digest.update(image.bytes.as_ref());
                let hash = digest
                    .finalize()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                let path = image_dir.join(format!("{hash}.png"));
                if !path.is_file() {
                    image::save_buffer_with_format(
                        &path,
                        image.bytes.as_ref(),
                        image.width as u32,
                        image.height as u32,
                        image::ColorType::Rgba8,
                        image::ImageFormat::Png,
                    )
                    .map_err(|e| format!("保存剪贴板图片失败：{e}"))?;
                }
                self.update_file(|data| {
                    capture_image(
                        data,
                        &hash,
                        &path,
                        image.width as u32,
                        image.height as u32,
                        now(),
                    );
                    Ok(())
                })?;
            } else {
                tracing::warn!("skipped clipboard image exceeding capture limits");
            }
        }
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
        let data = self
            .data
            .lock()
            .map_err(|_| "clipboard data unavailable")?
            .clone();
        let categories = data.categories;
        let items = data.items;
        *self
            .categories
            .lock()
            .map_err(|_| "clipboard categories unavailable")? = categories;
        *self
            .items
            .lock()
            .map_err(|_| "clipboard cache unavailable")? = items;
        self.cleanup_image_cache();
        Ok(())
    }
    fn cleanup_image_cache(&self) {
        let Ok(items) = self.snapshot() else {
            return;
        };
        let retained = items
            .into_iter()
            .filter_map(|item| item.image_path.map(PathBuf::from))
            .collect::<std::collections::HashSet<_>>();
        let Ok(directory) = image_store_directory() else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let generated_png = path.extension().is_some_and(|ext| ext == "png")
                && path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.len() == 64 && name.bytes().all(|b| b.is_ascii_hexdigit())
                    });
            if generated_png && !retained.contains(&path) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
    pub fn wake(&self) -> fileatlas_ui_bridge::refresh::Wake {
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
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        let selected = self
            .snapshot()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| "剪贴板记录已不存在".to_owned())?;
        let content = selected.content.clone();
        self.update_file(|data| {
            if let Some(item) = data.items.iter_mut().find(|item| item.id == id) {
                item.last_used_at = now();
            }
            Ok(())
        })?;
        if selected.content_type == "image" {
            let path = selected
                .image_path
                .ok_or_else(|| "历史图片文件缺失，无法复制".to_owned())?;
            let decoded = image::open(&path).map_err(|e| format!("读取历史图片失败：{e}"))?;
            let rgba = decoded.to_rgba8();
            clipboard
                .set_image(arboard::ImageData {
                    width: rgba.width() as usize,
                    height: rgba.height() as usize,
                    bytes: Cow::Owned(rgba.into_raw()),
                })
                .map_err(|e| e.to_string())?;
        } else {
            clipboard.set_text(content).map_err(|e| e.to_string())?;
        }
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
        self.update_file(|data| {
            if data
                .items
                .iter()
                .any(|item| item.id != id && item.content == content)
            {
                return Err("已有相同内容的剪贴板记录".into());
            }
            let item = data
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("剪贴板记录已不存在")?;
            item.content = content.to_owned();
            item.category_id = category;
            item.pinned |= pinned;
            item.last_used_at = now();
            Ok(())
        })?;
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
        let id = self.update_file(|data| {
            if let Some(item) = data.items.iter_mut().find(|item| item.content == content) {
                item.category_id = category;
                item.pinned |= pinned;
                item.last_used_at = now();
                return Ok(item.id);
            }
            let id = data.next_id;
            data.next_id += 1;
            data.items.push(ClipboardHistoryRecord {
                id,
                content: content.into(),
                content_type: "text".into(),
                image_path: None,
                image_width: None,
                image_height: None,
                category_id: category,
                pinned,
                created_at: now(),
                last_used_at: now(),
            });
            trim_history(data);
            Ok(id)
        })?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(id)
    }
    pub fn add_favorite(&self, id: i64) -> Result<(), String> {
        self.update_file(|data| {
            let item = data
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .ok_or("剪贴板记录已不存在")?;
            item.pinned = true;
            Ok(())
        })?;
        self.changed()
    }
    pub fn remove_favorite(&self, id: i64) -> Result<(), String> {
        self.update_file(|data| {
            let before = data.items.len();
            data.items.retain(|item| !(item.id == id && item.pinned));
            if before == data.items.len() {
                return Err("收藏记录已不存在".into());
            }
            Ok(())
        })?;
        self.changed()
    }
    pub fn delete(&self, id: i64) -> Result<(), String> {
        self.update_file(|data| {
            if data.items.iter().any(|item| item.id == id && item.pinned) {
                return Err("收藏内容请使用‘从收藏中删除’".into());
            };
            let before = data.items.len();
            data.items.retain(|item| item.id != id);
            if before == data.items.len() {
                return Err("剪贴板记录已不存在".into());
            }
            Ok(())
        })?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn clear_unpinned(&self) -> Result<(), String> {
        self.update_file(|data| {
            data.items.retain(|item| item.pinned);
            Ok(())
        })?;
        self.revision.fetch_add(1, Ordering::Release);
        self.wake.trigger();
        Ok(())
    }
    pub fn add_category(&self, name: &str) -> Result<(), String> {
        let name = name.trim();
        self.validate_group_name(name, None)?;
        self.update_file(|data| {
            let id = data
                .categories
                .iter()
                .map(|item| item.id)
                .max()
                .unwrap_or(0)
                + 1;
            data.categories.push(ClipboardCategoryRecord {
                id,
                name: name.into(),
            });
            Ok(())
        })?;
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
            .name("fileatlas-clipboard-listener".into())
            .spawn(move || {
                struct Handler(std::sync::Weak<ClipboardService>);
                impl ClipboardHandler for Handler {
                    fn on_clipboard_change(&mut self) -> CallbackResult {
                        if let Some(s) = self.0.upgrade() {
                            if let Err(e) = s.capture() {
                                tracing::debug!(%e,"clipboard content capture skipped");
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

fn image_store_directory() -> Result<PathBuf, String> {
    if let Some(profile) = std::env::var_os("FILEATLAS_DEV_TEST_DIR") {
        return Ok(PathBuf::from(profile).join("clipboard").join("images"));
    }
    dirs::data_local_dir()
        .map(|path| path.join("FileAtlas").join("clipboard").join("images"))
        .ok_or_else(|| "无法确定剪贴板图片缓存目录".into())
}
fn clipboard_json_path(db: &DatabaseService) -> Result<PathBuf, String> {
    if let Some(profile) = std::env::var_os("FILEATLAS_DEV_TEST_DIR") {
        return Ok(PathBuf::from(profile)
            .join("clipboard")
            .join("clipboard.json"));
    }
    Ok(db
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("clipboard")
        .join("clipboard.json"))
}
fn capture_text(data: &mut ClipboardFile, content: &str, timestamp: i64) {
    if let Some(item) = data.items.iter_mut().find(|item| item.content == content) {
        item.last_used_at = timestamp;
    } else {
        let id = data.next_id;
        data.next_id += 1;
        data.items.push(ClipboardHistoryRecord {
            id,
            content: content.into(),
            content_type: "text".into(),
            image_path: None,
            image_width: None,
            image_height: None,
            category_id: None,
            pinned: false,
            created_at: timestamp,
            last_used_at: timestamp,
        });
    }
    trim_history(data);
}
fn capture_image(
    data: &mut ClipboardFile,
    hash: &str,
    path: &std::path::Path,
    width: u32,
    height: u32,
    timestamp: i64,
) {
    let content = format!("image:{hash}");
    if let Some(item) = data.items.iter_mut().find(|item| item.content == content) {
        item.last_used_at = timestamp;
        item.image_path = Some(path.to_string_lossy().into_owned());
    } else {
        let id = data.next_id;
        data.next_id += 1;
        data.items.push(ClipboardHistoryRecord {
            id,
            content,
            content_type: "image".into(),
            image_path: Some(path.to_string_lossy().into_owned()),
            image_width: Some(width),
            image_height: Some(height),
            category_id: None,
            pinned: false,
            created_at: timestamp,
            last_used_at: timestamp,
        });
    }
    trim_history(data);
}
fn trim_history(data: &mut ClipboardFile) {
    let mut temporary = data
        .items
        .iter()
        .filter(|item| !item.pinned)
        .cloned()
        .collect::<Vec<_>>();
    temporary.sort_by_key(|item| std::cmp::Reverse(item.last_used_at));
    let keep = temporary
        .into_iter()
        .take(500)
        .map(|item| item.id)
        .collect::<std::collections::HashSet<_>>();
    data.items
        .retain(|item| item.pinned || keep.contains(&item.id));
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
            let data_path = clipboard_json_path(&db)?;
            let data = if data_path.is_file() {
                let loaded = serde_json::from_slice::<ClipboardFile>(
                    &std::fs::read(&data_path).map_err(|e| e.to_string())?,
                )
                .map_err(|e| format!("读取剪贴板 JSON 失败：{e}"))?;
                if loaded.version != 1 {
                    return Err(format!("不支持剪贴板数据版本：{}", loaded.version));
                }
                loaded
            } else {
                let (categories, mut items) = db.clipboard_snapshot().map_err(|e| e.to_string())?;
                let image_dir = image_store_directory()?;
                std::fs::create_dir_all(&image_dir).map_err(|e| e.to_string())?;
                let new_dir = image_dir.clone();
                for item in &mut items {
                    if let Some(old) = item.image_path.clone() {
                        let old = PathBuf::from(old);
                        if old.is_file() {
                            let name = old.file_name().ok_or("剪贴板图片路径无效")?;
                            let new = new_dir.join(name);
                            if old != new && !new.exists() {
                                std::fs::copy(&old, &new).map_err(|e| e.to_string())?;
                            }
                            item.image_path = Some(new.to_string_lossy().into_owned());
                        }
                    }
                }
                let data = ClipboardFile {
                    version: 1,
                    next_id: items.iter().map(|item| item.id).max().unwrap_or(0) + 1,
                    categories,
                    items,
                };
                data.persist(&data_path)?;
                data
            };
            let service = Arc::new(ClipboardService {
                data_path,
                data: Mutex::new(data.clone()),
                items: Mutex::new(data.items),
                categories: Mutex::new(data.categories),
                shutdown: Mutex::new(None),
                stopped: AtomicBool::new(false),
                paused: AtomicBool::new(false),
                revision: AtomicU64::new(0),
                wake: Default::default(),
            });
            let _publication = context
                .provide(Arc::clone(&service))
                .map_err(|e| e.to_string())?;
            mcp::register(&context, &service)?;
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
            let settings = context
                .try_service::<SettingsService>()
                .map_err(|e| e.to_string())?;
            let panels = Arc::clone(&bridge.panels);
            let command_panels = Arc::clone(&panels);
            let _listener = context
                .on::<fileatlas_ui_bridge::UiCommandInvoked, _>(cordis::event::observer_sync(
                    move |_, id| {
                        if id == OPEN {
                            let mode = settings
                                .snapshot()
                                .map(|settings| settings.plugin_launch_mode(OWNER))
                                .unwrap_or_default();
                            let _ = command_panels
                                .open_presented(PANEL, mode == PluginLaunchMode::PanelOnly);
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
                            needs: fileatlas_ui_bridge::CommandNeeds::None,
                        },
                    ),
                    UiContribution::new(
                        "clipboard-workbench/open-key",
                        ContributionKind::Keybinding {
                            key: "Ctrl+Shift+V".into(),
                            command_id: OPEN.into(),
                            scope: fileatlas_ui_bridge::ShortcutScope::System,
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
            fileatlas_ui_bridge::panel_catalog::register_panel_for_plugin(
                &context,
                OWNER,
                fileatlas_ui_bridge::panel_catalog::PanelDefinition {
                    options: fileatlas_ui_bridge::panels::PanelOptions {
                        id: PANEL.into(),
                        title: "剪贴板工作台".into(),
                        default_size: (1160., 760.),
                        min_size: (1000., 660.),
                    },
                    multiplicity: fileatlas_ui_bridge::panel_catalog::PanelMultiplicity::Single,
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
