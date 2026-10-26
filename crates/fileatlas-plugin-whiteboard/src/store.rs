//! Host-owned documents and durable metadata; scene semantics belong to Zorite.
use fileatlas_canvas_engine::{Scene, Template};
use fileatlas_ui_bridge::refresh::Wake;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
    time::Duration,
};

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: i64,
    pub title: String,
    /// Unmodified upstream Scene JSON, also used by native export.
    pub scene: String,
    #[serde(default)]
    pub font_path: Option<String>,
    #[serde(default)]
    pub toolbar_pos: Option<(f32, f32)>,
    #[serde(default)]
    pub toolbar_vertical: bool,
    #[serde(default)]
    pub files: Vec<LinkedFile>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct LinkedFile {
    pub id: i64,
    #[serde(alias = "path")]
    pub uri: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedTemplate {
    pub id: i64,
    pub name: String,
    pub elements: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub next_id: i64,
    pub active_id: Option<i64>,
    pub boards: Vec<Board>,
    #[serde(default)]
    pub templates: Vec<SavedTemplate>,
    #[serde(default)]
    pub colors: Vec<u32>,
    #[serde(default)]
    pub links: Vec<LinkedFile>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 2,
            next_id: 2,
            active_id: Some(1),
            boards: vec![Board {
                id: 1,
                title: "画板 1".into(),
                scene: Scene::default().to_json(),
                font_path: None,
                toolbar_pos: None,
                toolbar_vertical: false,
                files: vec![],
            }],
            templates: vec![],
            colors: vec![],
            links: vec![],
        }
    }
}
#[derive(Clone, Default, Serialize, PartialEq, Eq)]
pub struct SaveStatus {
    pub revision: u64,
    pub saved_revision: u64,
    pub error: Option<String>,
}
struct Shared {
    library: Mutex<Library>,
    status: Mutex<SaveStatus>,
    wake: Wake,
}
enum Write {
    Save(u64, Library),
    Flush(mpsc::Sender<Result<()>>),
}
pub struct WhiteboardService {
    pub root: PathBuf,
    shared: Arc<Shared>,
    writer: Option<mpsc::Sender<Write>>,
    worker: Option<JoinHandle<()>>,
    stopped: std::sync::atomic::AtomicBool,
    images: Mutex<Vec<(i64, Vec<PathBuf>, Option<[f32; 2]>)>>,
}
impl cordis::Service for WhiteboardService {
    const NAME: &'static str = "fileatlas/whiteboard";
}
impl Drop for WhiteboardService {
    fn drop(&mut self) {
        self.writer.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join(); // disconnect drains and flushes the latest document
        }
    }
}
impl WhiteboardService {
    pub fn open(root: PathBuf) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let path = root.join("library.json");
        let library = load_library(&path)?;
        let shared = Arc::new(Shared {
            library: Mutex::new(library),
            status: Mutex::new(SaveStatus::default()),
            wake: Wake::default(),
        });
        let (send, recv) = mpsc::channel();
        let state = Arc::clone(&shared);
        let worker = std::thread::Builder::new()
            .name("whiteboard-save".into())
            .spawn(move || writer_loop(recv, state, path))
            .map_err(|e| e.to_string())?;
        Ok(Arc::new(Self {
            root,
            shared,
            writer: Some(send),
            worker: Some(worker),
            stopped: std::sync::atomic::AtomicBool::new(false),
            images: Mutex::new(vec![]),
        }))
    }
    pub fn snapshot(&self) -> Library {
        self.shared
            .library
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
    pub fn status(&self) -> SaveStatus {
        self.shared
            .status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
    pub fn wake(&self) -> Wake {
        self.shared.wake.clone()
    }
    pub fn queue_images(&self, id: i64, paths: Vec<PathBuf>, at: Option<[f32; 2]>) -> Result<()> {
        self.activate(id)?;
        self.images
            .lock()
            .map_err(|e| e.to_string())?
            .push((id, paths, at));
        self.shared.wake.trigger();
        Ok(())
    }
    pub fn take_images(&self) -> Vec<(i64, Vec<PathBuf>, Option<[f32; 2]>)> {
        self.images
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }
    pub fn board(&self, id: i64) -> Result<Board> {
        self.snapshot()
            .boards
            .into_iter()
            .find(|b| b.id == id)
            .ok_or_else(|| "画板不存在".into())
    }
    fn change<T>(&self, change: impl FnOnce(&mut Library) -> Result<T>) -> Result<T> {
        let mut state = self.shared.library.lock().map_err(|e| e.to_string())?;
        if self.stopped.load(std::sync::atomic::Ordering::Acquire) {
            return Err("画板插件已停用".into());
        }
        let mut next = state.clone();
        let result = change(&mut next)?;
        if serde_json::to_string(&next).map_err(|e| e.to_string())?
            == serde_json::to_string(&*state).map_err(|e| e.to_string())?
        {
            return Ok(result);
        }
        let mut status = self.shared.status.lock().map_err(|e| e.to_string())?;
        status.revision += 1;
        self.writer
            .as_ref()
            .ok_or("保存线程已关闭")?
            .send(Write::Save(status.revision, next.clone()))
            .map_err(|e| e.to_string())?;
        *state = next;
        drop(status);
        drop(state);
        self.shared.wake.trigger();
        Ok(result)
    }
    pub fn create(&self, title: &str) -> Result<i64> {
        self.change(|s| {
            let id = s.next_id;
            s.next_id += 1;
            s.boards.push(Board {
                id,
                title: title_or(title, format!("画板 {id}")),
                scene: Scene::default().to_json(),
                font_path: None,
                toolbar_pos: None,
                toolbar_vertical: false,
                files: vec![],
            });
            s.active_id = Some(id);
            Ok(id)
        })
    }
    pub fn activate(&self, id: i64) -> Result<()> {
        self.board(id)?;
        self.change(|s| {
            s.active_id = Some(id);
            Ok(())
        })
    }
    pub fn update_board(&self, id: i64, f: impl FnOnce(&mut Board)) -> Result<()> {
        self.change(|s| {
            let b = s
                .boards
                .iter_mut()
                .find(|b| b.id == id)
                .ok_or("画板不存在")?;
            f(b);
            Ok(())
        })
    }
    pub fn rename(&self, id: i64, title: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Err("画板名称不能为空".into());
        }
        self.update_board(id, |b| b.title = title.trim().into())
    }
    pub fn remove(&self, id: i64) -> Result<()> {
        self.board(id)?;
        self.change(|s| {
            s.boards.retain(|b| b.id != id);
            if s.active_id == Some(id) {
                s.active_id = s.boards.first().map(|b| b.id);
            }
            Ok(())
        })
    }
    pub fn save_scene(&self, id: i64, scene: String) -> Result<()> {
        serde_json::from_str::<Scene>(&scene).map_err(|e| e.to_string())?;
        self.update_board(id, |b| b.scene = scene)
    }
    pub fn import(&self, path: &Path) -> Result<i64> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if !value.get("elements").is_some_and(|v| v.is_array()) {
            return Err("所选文件不包含 Zorite Scene elements".into());
        }
        let scene: Scene = serde_json::from_str(&text)
            .map_err(|e| format!("不是有效的 Zorite Scene JSON：{e}"))?;
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("导入画板");
        self.change(|s| {
            let id = s.next_id;
            s.next_id += 1;
            s.boards.push(Board {
                id,
                title: title.to_owned(),
                scene: scene.to_json(),
                font_path: None,
                toolbar_pos: None,
                toolbar_vertical: false,
                files: vec![],
            });
            s.active_id = Some(id);
            Ok(id)
        })
    }
    pub fn export(&self, id: i64, path: &Path) -> Result<()> {
        atomic_write(path, self.board(id)?.scene.as_bytes())
    }
    pub fn link(&self, id: i64, path: &Path) -> Result<i64> {
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if !path.is_file() {
            return Err("请选择文件".into());
        }
        self.link_resource(id, &path.to_string_lossy())
    }
    pub fn link_resource(&self, id: i64, uri: &str) -> Result<i64> {
        if uri.trim().is_empty() {
            return Err("资源链接不能为空".into());
        }
        self.change(|s| {
            let file_id = s.next_id;
            s.next_id += 1;
            let board = s
                .boards
                .iter_mut()
                .find(|b| b.id == id)
                .ok_or("画板不存在")?;
            let file = LinkedFile {
                id: file_id,
                uri: uri.to_owned(),
            };
            board.files.push(file.clone());
            s.links.push(file);
            Ok(file_id)
        })
    }
    pub fn linked_file(&self, id: i64) -> Option<String> {
        self.snapshot()
            .links
            .into_iter()
            .find(|f| f.id == id)
            .map(|f| f.uri)
    }
    pub fn templates(&self) -> Vec<Template> {
        self.snapshot()
            .templates
            .into_iter()
            .map(|t| Template::from_json(t.id, t.name, &t.elements))
            .collect()
    }
    pub fn save_template(&self, name: String, elements: String) -> Result<()> {
        serde_json::from_str::<Vec<fileatlas_canvas_engine::Element>>(&elements)
            .map_err(|e| e.to_string())?;
        self.change(|s| {
            let id = s.next_id;
            s.next_id += 1;
            s.templates.push(SavedTemplate { id, name, elements });
            Ok(())
        })
    }
    pub fn delete_template(&self, id: i64) -> Result<()> {
        self.change(|s| {
            s.templates.retain(|t| t.id != id);
            Ok(())
        })
    }
    pub fn save_colors(&self, colors: Vec<u32>) -> Result<()> {
        self.change(|s| {
            s.colors = colors;
            Ok(())
        })
    }
    pub fn flush(&self) -> Result<()> {
        let (send, receive) = mpsc::channel();
        self.writer
            .as_ref()
            .ok_or("保存线程已关闭")?
            .send(Write::Flush(send))
            .map_err(|e| e.to_string())?;
        receive.recv().map_err(|e| e.to_string())?
    }
    pub fn stop(&self) -> Result<()> {
        let guard = self.shared.library.lock().map_err(|e| e.to_string())?;
        self.stopped
            .store(true, std::sync::atomic::Ordering::Release);
        drop(guard);
        self.flush()
    }
}
fn title_or(title: &str, fallback: String) -> String {
    if title.trim().is_empty() {
        fallback
    } else {
        title.trim().to_owned()
    }
}
fn validate(text: &str) -> Result<Library> {
    let s: Library = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if !(1..=2).contains(&s.version) {
        return Err(format!("不支持的画板清单版本 {}", s.version));
    }
    let mut s = s;
    // Version 2 replaces host-only absolute path fields with provider URIs.
    // `LinkedFile.uri` accepts the v1 `path` key as a lossless migration alias.
    s.version = 2;
    for b in &s.boards {
        serde_json::from_str::<Scene>(&b.scene)
            .map_err(|e| format!("画板 {} 数据无效：{e}", b.title))?;
    }
    Ok(s)
}
fn load_library(path: &Path) -> Result<Library> {
    if !path.exists() && !path.with_extension("json.bak").exists() {
        return Ok(Library::default());
    }
    // A newer valid format is not corruption: never roll it back to an old backup.
    if let Ok(text) = std::fs::read_to_string(path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if value
                .get("version")
                .and_then(|v| v.as_u64())
                .is_some_and(|v| v > 2)
            {
                return Err("画板清单版本较新，请使用兼容版本".into());
            }
        }
    }
    match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|t| validate(&t))
    {
        Ok(s) => Ok(s),
        Err(primary) => {
            let backup = std::fs::read_to_string(path.with_extension("json.bak"))
                .map_err(|e| e.to_string())
                .and_then(|t| validate(&t));
            if backup.is_ok() {
                if path.exists() {
                    std::fs::copy(path, path.with_extension("json.corrupt"))
                        .map_err(|e| e.to_string())?;
                }
                std::fs::copy(path.with_extension("json.bak"), path).map_err(|e| e.to_string())?;
                tracing::warn!(%primary, "whiteboard library recovered from backup; damaged primary retained as .corrupt");
            }
            backup.map_err(|backup| format!("画板清单无法读取：{primary}；备份：{backup}"))
        }
    }
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    if path.is_file() {
        let backup = path.with_extension("json.bak");
        std::fs::copy(path, backup).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&temporary, path).map_err(|e| e.to_string())
}
fn writer_loop(recv: mpsc::Receiver<Write>, shared: Arc<Shared>, path: PathBuf) {
    let mut pending = None;
    loop {
        match recv.recv_timeout(Duration::from_millis(250)) {
            Ok(Write::Save(revision, doc)) => pending = Some((revision, doc)),
            Ok(Write::Flush(done)) => {
                if pending.is_none() {
                    if let Ok(library) = shared.library.lock() {
                        if let Ok(status) = shared.status.lock() {
                            if status.error.is_some() {
                                pending = Some((status.revision, library.clone()));
                            }
                        }
                    }
                }
                let _ = done.send(write_pending(&mut pending, &shared, &path));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = write_pending(&mut pending, &shared, &path);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = write_pending(&mut pending, &shared, &path);
                break;
            }
        }
    }
}
fn write_pending(pending: &mut Option<(u64, Library)>, shared: &Shared, path: &Path) -> Result<()> {
    let Some((revision, doc)) = pending.as_ref() else {
        return Ok(());
    };
    let result = serde_json::to_vec_pretty(doc)
        .map_err(|e| e.to_string())
        .and_then(|bytes| atomic_write(path, &bytes));
    if let Ok(mut status) = shared.status.lock() {
        let previous = status.clone();
        status.error = result.as_ref().err().cloned();
        if result.is_ok() {
            status.saved_revision = *revision;
        }
        if *status != previous {
            shared.wake.trigger();
        }
    }
    // An explicit edit/flush retries after a failure; idle must not repeat IO/logs.
    pending.take();
    result
}

#[cfg(test)]
mod tests;
