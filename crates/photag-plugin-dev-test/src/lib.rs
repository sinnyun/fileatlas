//! Development-only tools for exercising the real PhoTag scanner and large
//! Gallery data sets without creating large media files on disk.

mod benchmark;
mod view;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cordis::{Context, Plugin, Service};
use photag_contracts::{FolderSelectionChanged, MediaIndexChanged};
use photag_plugin_database::DatabaseService;
use photag_plugin_indexing::IndexingService;
use photag_ui_bridge::register_views_for_plugin;
use tokio::runtime::Handle;

const OWNER: &str = "photag.dev-test";
const VIRTUAL_DIR: &str = ".photag-virtual-benchmark";
const MAX_VIRTUAL_RECORDS: usize = 100_000;

pub struct DevTestService {
    database: Arc<DatabaseService>,
    indexing: Arc<IndexingService>,
    status: Mutex<String>,
    running: AtomicBool,
    context: Context,
    runtime: Handle,
}

impl Service for DevTestService {
    const NAME: &'static str = "photag/dev-test";
}

impl DevTestService {
    fn fixture_root() -> Result<PathBuf, DevTestError> {
        let configured = std::env::var_os("PHOTAG_TEST_LIBRARY").map(PathBuf::from);
        let root = configured.unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-data/photag-library")
        });
        let root = root.canonicalize().map_err(DevTestError::Io)?;
        if !root.is_dir() {
            return Err(DevTestError::FixtureMissing(root));
        }
        Ok(root)
    }

    pub fn status(&self) -> String {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| "开发测试状态不可用（状态锁异常）".into())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    fn set_status(&self, status: impl Into<String>) {
        if let Ok(mut current) = self.status.lock() {
            *current = status.into();
        }
    }

    fn begin(&self, action: &str) -> bool {
        if self
            .running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            self.set_status("已有开发测试任务运行中，请等待完成。");
            return false;
        }
        self.set_status(format!("{action}：准备中…"));
        true
    }

    fn finish(&self, status: impl Into<String>) {
        self.set_status(status);
        self.running.store(false, Ordering::Release);
    }

    pub fn scan_fixture(self: &Arc<Self>) -> bool {
        if !self.begin("扫描样例目录") {
            return false;
        }
        let service = Arc::clone(self);
        let spawn = std::thread::Builder::new()
            .name("photag-dev-fixture-scan".into())
            .spawn(move || {
                let result = service.scan_fixture_blocking();
                match result {
                    Ok((folder_id, count, elapsed)) => {
                        service.finish(format!(
                            "样例扫描完成：folder_id={folder_id}，处理 {count} 项，用时 {elapsed:.2?}。"
                        ));
                        service.emit_index_changed(folder_id);
                    }
                    Err(error) => service.finish(format!("样例扫描失败：{error}")),
                }
            });
        if let Err(error) = spawn {
            self.finish(format!("无法启动扫描线程：{error}"));
            return false;
        }
        true
    }

    fn scan_fixture_blocking(
        self: &Arc<Self>,
    ) -> Result<(i64, usize, std::time::Duration), DevTestError> {
        let root = Self::fixture_root()?;
        let folder = self.database.add_folder(&root.to_string_lossy())?;
        let cancel = Arc::new(AtomicBool::new(false));
        let receiver = self
            .indexing
            .scan_folder(folder.id, folder.path.clone(), cancel);
        let started = Instant::now();
        while let Ok(message) = receiver.recv() {
            let Some(progress) = message else { break };
            self.set_status(format!(
                "样例目录扫描中：{}（{}/{}）",
                progress.phase, progress.scanned, progress.total
            ));
        }
        let count = self.database.count_media_in_folder(folder.id)?;
        Ok((folder.id, count as usize, started.elapsed()))
    }

    pub fn inject_virtual(self: &Arc<Self>, count: usize) -> bool {
        if !matches!(count, 1_000 | 10_000 | 100_000) {
            self.set_status("仅支持注入 1,000、10,000 或 100,000 条虚拟记录。");
            return false;
        }
        if !self.begin(&format!("注入 {count} 条虚拟记录")) {
            return false;
        }
        let service = Arc::clone(self);
        let spawn = std::thread::Builder::new()
            .name("photag-dev-virtual-seed".into())
            .spawn(move || match service.inject_virtual_blocking(count) {
                Ok((elapsed, folder_id)) => {
                    service.finish(format!(
                        "已注入 {count} 条虚拟记录，用时 {elapsed:.2?}。虚拟行不对应磁盘文件。"
                    ));
                    service.emit_index_changed(folder_id);
                }
                Err(error) => service.finish(format!("虚拟数据注入失败：{error}")),
            });
        if let Err(error) = spawn {
            self.finish(format!("无法启动注入线程：{error}"));
            return false;
        }
        true
    }

    fn inject_virtual_blocking(
        &self,
        count: usize,
    ) -> Result<(std::time::Duration, i64), DevTestError> {
        debug_assert!(count <= MAX_VIRTUAL_RECORDS);
        let root = Self::fixture_root()?;
        let preview = root.join(".dev-support").join("preview.png");
        benchmark::replace_virtual_records(&self.database, &root, &preview, count, |inserted| {
            self.set_status(format!("正在注入虚拟记录：{inserted}/{count}…"))
        })
    }

    pub fn clear_virtual(self: &Arc<Self>) -> bool {
        if !self.begin("清理虚拟记录") {
            return false;
        }
        let service = Arc::clone(self);
        let spawn = std::thread::Builder::new()
            .name("photag-dev-virtual-cleanup".into())
            .spawn(move || {
                let result = Self::fixture_root()
                    .and_then(|root| benchmark::clear_virtual_records(&service.database, &root));
                match result {
                    Ok((removed, folder_id)) => {
                        service.finish(format!("已清理 {removed} 条虚拟记录。"));
                        service.emit_index_changed(folder_id);
                    }
                    Err(error) => service.finish(format!("清理虚拟记录失败：{error}")),
                }
            });
        if let Err(error) = spawn {
            self.finish(format!("无法启动清理线程：{error}"));
            return false;
        }
        true
    }

    fn emit_index_changed(&self, folder_id: i64) {
        let context = self.context.clone();
        self.runtime.spawn(async move {
            if let Err(error) = context
                .emit::<FolderSelectionChanged>(cordis::Routing::Unscoped, Some(folder_id))
                .await
            {
                tracing::error!("dev test folder selection event failed: {error}");
            }
            if let Err(error) = context
                .emit::<MediaIndexChanged>(cordis::Routing::Unscoped, ())
                .await
            {
                tracing::error!("dev test media refresh event failed: {error}");
            }
        });
    }
}

pub struct DevTestPlugin;

#[derive(Debug, Clone, Default)]
pub struct DevTestConfig;

impl Plugin for DevTestPlugin {
    type Config = DevTestConfig;
    type Input = DevTestConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = DevTestPluginError;

    fn name(&self) -> std::borrow::Cow<'_, str> {
        OWNER.into()
    }

    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }

    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, _config: &Self::Input) -> Result<(), Self::ApplyError> {
        let database = context
            .try_service::<DatabaseService>()
            .map_err(|error| DevTestPluginError::Dependency(error.to_string()))?;
        let indexing = context
            .try_service::<IndexingService>()
            .map_err(|error| DevTestPluginError::Dependency(error.to_string()))?;
        let service = Arc::new(DevTestService {
            database,
            indexing,
            status: Mutex::new(format!(
                "就绪。样例目录：{}",
                DevTestService::fixture_root()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|_| "设置 PHOTAG_TEST_LIBRARY 指定目录".into())
            )),
            running: AtomicBool::new(false),
            context: context.clone(),
            runtime: Handle::current(),
        });
        let _publication = context
            .provide(Arc::clone(&service))
            .map_err(|error| DevTestPluginError::Publish(error.to_string()))?;
        for (id, multiplicity, size) in [
            (
                "dev-test/status",
                photag_ui_bridge::panel_catalog::PanelMultiplicity::Single,
                (520., 300.),
            ),
            (
                "dev-test/status-multi",
                photag_ui_bridge::panel_catalog::PanelMultiplicity::Multiple,
                (600., 360.),
            ),
        ] {
            let service = service.clone();
            photag_ui_bridge::panel_catalog::register_panel_for_plugin(
                &context,
                OWNER,
                photag_ui_bridge::panel_catalog::PanelDefinition {
                    options: photag_ui_bridge::panels::PanelOptions::new(
                        id,
                        "Development task status",
                        size,
                    ),
                    multiplicity,
                    factory: Arc::new(move |_, app| view::status_panel(service.clone(), app)),
                },
            )
            .map_err(DevTestPluginError::UiRegistration)?;
        }
        register_views_for_plugin(&context, OWNER, [view::dev_test_panel(service)])
            .map_err(|error| DevTestPluginError::UiRegistration(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DevTestError {
    #[error("test media directory does not exist: {0}")]
    FixtureMissing(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] photag_plugin_database::DatabaseAccessError),
}

#[derive(Debug, thiserror::Error)]
pub enum DevTestPluginError {
    #[error("required dev-test dependency unavailable: {0}")]
    Dependency(String),
    #[error("dev-test service publication failed: {0}")]
    Publish(String),
    #[error("dev-test panel registration failed: {0}")]
    UiRegistration(String),
}

#[cfg(test)]
mod tests {
    use super::benchmark::{clear_virtual_records, replace_virtual_records};
    use super::*;
    use photag_plugin_database::queries;
    use photag_plugin_thumbnails::ThumbnailService;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "photag-dev-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn generated_media_folder_scans_nested_formats_and_missing_thumbnails() {
        let root = temp_root("scan");
        let cache = root.join("cache");
        let database = Arc::new(DatabaseService::open_at(root.join("photag.sqlite")).unwrap());
        let thumbnails = Arc::new(ThumbnailService::open_at(&cache).unwrap());
        let indexing = Arc::new(IndexingService::new(Arc::clone(&database), thumbnails));
        let fixture = DevTestService::fixture_root().unwrap();
        let folder = database
            .with_writer(|connection| queries::add_folder(connection, &fixture.to_string_lossy()))
            .unwrap();
        let receiver =
            indexing.scan_folder(folder.id, folder.path, Arc::new(AtomicBool::new(false)));
        while let Ok(progress) = receiver.recv() {
            if progress.is_none() {
                break;
            }
        }

        let (total, with_thumbnail, audio_count): (i64, i64, i64) = database
            .with_reader(|connection| {
                connection.query_row(
                    "SELECT COUNT(*),
                            SUM(CASE WHEN thumb_path IS NOT NULL THEN 1 ELSE 0 END),
                            SUM(CASE WHEN media_type = 'audio' THEN 1 ELSE 0 END)
                     FROM media WHERE folder_id = ?1",
                    [folder.id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
            })
            .unwrap();
        assert_eq!(
            total, 12,
            "all images, WAV, README, XMP and hidden support image"
        );
        assert_eq!(audio_count, 1);
        assert!(
            with_thumbnail >= 5,
            "valid formats should produce cached thumbnails"
        );
        assert!(
            with_thumbnail < total,
            "broken image and audio have no thumbnail"
        );
        assert!(cache.read_dir().unwrap().next().is_some());
        drop(indexing);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn virtual_benchmark_replaces_and_cleans_one_hundred_thousand_rows() {
        let root = temp_root("virtual");
        std::fs::create_dir_all(&root).unwrap();
        let database = DatabaseService::open_at(root.join("photag.sqlite")).unwrap();
        let preview = DevTestService::fixture_root()
            .unwrap()
            .join(".dev-support/preview.png");
        replace_virtual_records(&database, &root, &preview, 1_000, |_| {}).unwrap();
        let mut progress_max = 0;
        replace_virtual_records(&database, &root, &preview, MAX_VIRTUAL_RECORDS, |value| {
            progress_max = value;
        })
        .unwrap();

        let (total, with_thumbnail, video_count, audio_count): (i64, i64, i64, i64) = database
            .with_reader(|connection| {
                connection.query_row(
                    "SELECT COUNT(*),
                            SUM(CASE WHEN thumb_path IS NOT NULL THEN 1 ELSE 0 END),
                            SUM(CASE WHEN media_type = 'video' THEN 1 ELSE 0 END),
                            SUM(CASE WHEN media_type = 'audio' THEN 1 ELSE 0 END)
                     FROM media
                     WHERE folder_id = (SELECT id FROM folders WHERE path = ?1)",
                    [root.to_string_lossy().as_ref()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
            })
            .unwrap();
        assert_eq!(total, MAX_VIRTUAL_RECORDS as i64);
        assert_eq!(with_thumbnail, 25_000);
        assert_eq!(video_count, 14_285);
        assert_eq!(audio_count, 14_285);
        assert_eq!(progress_max, MAX_VIRTUAL_RECORDS);
        assert!(
            !root.join(VIRTUAL_DIR).exists(),
            "virtual rows must not create physical files"
        );
        assert_eq!(clear_virtual_records(&database, &root).unwrap().0, 100_000);
        assert_eq!(clear_virtual_records(&database, &root).unwrap().0, 0);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn browser_virtual_rows_do_not_persist_and_legacy_rows_are_removed()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_root("browser-virtual");
        std::fs::create_dir_all(&root)?;
        let path = root.join("photag.sqlite");
        let marker =
            root.join(VIRTUAL_DIR).to_string_lossy().into_owned() + std::path::MAIN_SEPARATOR_STR;
        let preview = DevTestService::fixture_root()?.join(".dev-support/preview.png");

        // Simulate the old behavior where browser-mode writer persistence
        // retained rated synthetic rows between application launches.
        let database = DatabaseService::open_browser_at(&path)?;
        let folder = database.add_folder(&root.to_string_lossy())?;
        let legacy_path = format!("{marker}virtual-000001.png");
        database.with_writer(|connection| {
            let media_id = queries::insert_media(
                connection,
                folder.id,
                "virtual-000001.png",
                &legacy_path,
                "image",
                10,
                1,
                None,
                None,
                None,
            )?;
            connection.execute("UPDATE media SET rating=1 WHERE id=?1", [media_id])?;
            Ok(())
        })?;
        drop(database);

        let database = DatabaseService::open_browser_at(&path)?;
        assert_eq!(database.count_media_in_folder(folder.id)?, 1);
        replace_virtual_records(&database, &root, &preview, 1_000, |_| {})?;
        assert_eq!(database.count_media_in_folder(folder.id)?, 1_000);
        drop(database);

        let database = DatabaseService::open_browser_at(&path)?;
        assert_eq!(database.count_media_in_folder(folder.id)?, 0);
        replace_virtual_records(&database, &root, &preview, 1_000, |_| {})?;
        assert_eq!(database.count_media_in_folder(folder.id)?, 1_000);
        clear_virtual_records(&database, &root)?;
        drop(database);
        let database = DatabaseService::open_browser_at(&path)?;
        assert_eq!(database.count_media_in_folder(folder.id)?, 0);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
        Ok(())
    }
}
