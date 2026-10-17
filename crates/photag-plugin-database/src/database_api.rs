//! Typed operations exposed by DatabaseService. Query implementations remain
//! private to this crate; consumers cannot provide arbitrary SQLite callbacks
//! for routine operations.
use crate::db::queries::{self, *};
use crate::{
    BrowserFile, DatabaseAccessError, DatabaseService, GalleryDatabaseSnapshot, ScanFileRecord,
    TaggedMediaSourceUpdate, ThumbnailRecord, WatchedFile,
};
use rusqlite::Connection;

macro_rules! read_method {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?) -> $result:ty) => {
        pub fn $name(&self, $($arg: $ty),*) -> Result<$result, DatabaseAccessError> {
            self.with_reader(|connection| queries::$name(connection, $($arg),*))
        }
    };
}

macro_rules! write_method {
    ($name:ident($($arg:ident: $ty:ty),* $(,)?) -> $result:ty) => {
        pub fn $name(&self, $($arg: $ty),*) -> Result<$result, DatabaseAccessError> {
            self.with_writer(|connection| queries::$name(connection, $($arg),*))
        }
    };
}

impl DatabaseService {
    /// Refresh filesystem-derived columns after an explicit version restore.
    /// Annotation links, rating, capture date and stable row identity remain intact.
    pub fn refresh_restored_file(
        &self,
        media_id: i64,
        media_type: &str,
        file_size: i64,
        modified_at: i64,
        thumbnail: Option<(&str, i64, i64, &[u8])>,
    ) -> Result<(), DatabaseAccessError> {
        let update = |connection: &Connection| {
            let (path, width, height, micro) = thumbnail
                .map(|(path, width, height, micro)| {
                    (Some(path), Some(width), Some(height), Some(micro))
                })
                .unwrap_or((None, None, None, None));
            connection.execute(
                "UPDATE media SET media_type=?1,file_size=?2,modified_at=?3,thumb_path=?4,width=?5,height=?6,micro_thumb=?7 WHERE id=?8",
                rusqlite::params![media_type,file_size,modified_at,path,width,height,micro,media_id],
            )?;
            Ok(())
        };
        if self.browser {
            self.with_cache_writer(update)
        } else {
            self.with_writer(update)
        }
    }

    pub fn count_media_in_folder(&self, folder_id: i64) -> Result<i64, DatabaseAccessError> {
        self.with_reader(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM media WHERE folder_id=?1",
                [folder_id],
                |row| row.get(0),
            )
        })
    }

    pub fn remove_live_path(&self, full_path: &str) -> Result<(), DatabaseAccessError> {
        self.with_cache_writer(|connection| {
            connection.execute("DELETE FROM live_paths WHERE full_path=?1", [full_path])?;
            Ok(())
        })
    }

    /// Atomically replace benchmark rows owned by the development fixture.
    pub fn replace_virtual_benchmark(
        &self,
        root: &str,
        marker: &str,
        preview: &str,
        count: usize,
        mut progress: impl FnMut(usize),
    ) -> Result<i64, DatabaseAccessError> {
        self.with_cache_writer(|connection| {
            let folder = queries::add_folder(connection, root)?;
            let transaction = connection.unchecked_transaction()?;
            transaction.execute("DELETE FROM media WHERE folder_id=?1 AND substr(full_path,1,length(?2))=?2", rusqlite::params![folder.id, marker])?;
            // A previous browser-mode run may have persisted rated synthetic
            // rows. Remove the reserved prefix from durable storage before
            // reseeding this session-only benchmark.
            if self.browser {
                transaction.execute(
                    "DELETE FROM saved.media WHERE substr(full_path,1,length(?1))=?1",
                    [marker],
                )?;
            }
            {
                let mut insert = transaction.prepare_cached("INSERT INTO media(folder_id,filename,full_path,media_type,file_size,modified_at,width,height,thumb_path,rating) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)")?;
                for index in 0..count {
                    let extension = match index % 7 { 0 => "jpg", 1 => "png", 2 => "webp", 3 => "tif", 4 => "bmp", 5 => "mp4", _ => "wav" };
                    let media_type = match extension { "mp4" => "video", "wav" => "audio", _ => "image" };
                    let filename = format!("virtual-{index:06}.{extension}");
                    let full_path = format!("{marker}{filename}");
                    let thumb = (index % 4 == 0).then_some(preview);
                    let side = 256 + (index % 3_841) as i64;
                    let file_size = 512 + ((index * 7919) % (32 * 1024 * 1024)) as i64;
                    insert.execute(rusqlite::params![folder.id, filename, full_path, media_type, file_size, 1_700_000_000i64 + (index % 1_000_000) as i64, side, side * 3 / 4, thumb, (index % 6) as i64])?;
                    if (index + 1) % 10_000 == 0 { progress(index + 1); }
                }
            }
            transaction.commit()?;
            Ok(folder.id)
        })
    }

    pub fn clear_virtual_benchmark(
        &self,
        root: &str,
        marker: &str,
    ) -> Result<(usize, i64), DatabaseAccessError> {
        self.with_cache_writer(|connection| {
            let folder = queries::add_folder(connection, root)?;
            let removed = connection.execute(
                "DELETE FROM media WHERE folder_id=?1 AND substr(full_path,1,length(?2))=?2",
                rusqlite::params![folder.id, marker],
            )?;
            if self.browser {
                connection.execute(
                    "DELETE FROM saved.media WHERE substr(full_path,1,length(?1))=?1",
                    [marker],
                )?;
            }
            Ok((removed, folder.id))
        })
    }

    pub fn set_favorite_name(&self, id: i64, name: &str) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute("INSERT INTO favorite_meta(id,name,position) VALUES(?1,?2,?1) ON CONFLICT(id) DO UPDATE SET name=excluded.name", rusqlite::params![id,name])?;
            Ok(())
        })
    }

    pub fn set_favorite_order(&self, entries: &[(i64, String)]) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            for (position, (id, name)) in entries.iter().enumerate() {
                connection.execute("INSERT INTO favorite_meta(id,name,position) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET position=excluded.position", rusqlite::params![id,name,position as i64])?;
            }
            Ok(())
        })
    }

    pub fn annotated_media(&self) -> Result<Vec<MediaItem>, DatabaseAccessError> {
        self.with_reader(|connection| {
            let ids = {
                let mut statement =
                    connection.prepare("SELECT DISTINCT media_id FROM media_tags")?;
                statement
                    .query_map([], |row| row.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            ids.iter()
                .map(|id| queries::get_media_by_id(connection, *id))
                .collect::<rusqlite::Result<Vec<_>>>()
                .map(|items| items.into_iter().flatten().collect())
        })
    }

    pub fn annotated_media_with_previews(
        &self,
    ) -> Result<Vec<(MediaItem, Option<String>)>, DatabaseAccessError> {
        self.with_reader(|connection| {
            let ids = {
                let mut statement =
                    connection.prepare("SELECT DISTINCT media_id FROM media_tags")?;
                statement
                    .query_map([], |row| row.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            let mut preview = if self.browser {
                Some(connection.prepare("SELECT thumb_path FROM tag_previews WHERE media_id=?1")?)
            } else {
                None
            };
            let mut result = Vec::new();
            for id in ids {
                if let Some(media) = queries::get_media_by_id(connection, id)? {
                    let path = preview.as_mut().and_then(|statement| {
                        statement
                            .query_row([id], |row| row.get::<_, String>(0))
                            .ok()
                    });
                    result.push((media, path));
                }
            }
            Ok(result)
        })
    }

    pub fn tag_preview_path(&self, media_id: i64) -> Result<Option<String>, DatabaseAccessError> {
        if !self.browser {
            return Ok(None);
        }
        self.with_reader(|connection| {
            Ok(connection
                .query_row(
                    "SELECT thumb_path FROM tag_previews WHERE media_id=?1",
                    [media_id],
                    |row| row.get::<_, String>(0),
                )
                .ok())
        })
    }

    pub fn update_tagged_media_source(
        &self,
        update: &TaggedMediaSourceUpdate,
    ) -> Result<(), DatabaseAccessError> {
        self.with_cache_writer(|connection| {
            connection.execute("UPDATE media SET file_size=?1,modified_at=?2 WHERE id=?3", rusqlite::params![update.file_size,update.modified_at,update.media_id])?;
            if let Some(path) = &update.preview_path {
                connection.execute("INSERT OR REPLACE INTO tag_previews VALUES(?1,?2)", rusqlite::params![update.media_id,path])?;
            }
            if let Some(thumb) = &update.thumbnail {
                connection.execute("UPDATE media SET thumb_path=?1,width=COALESCE(?2,width),height=COALESCE(?3,height),micro_thumb=?4 WHERE id=?5", rusqlite::params![thumb.thumb_path,thumb.width,thumb.height,thumb.micro_thumb,update.media_id])?;
            }
            Ok(())
        })
    }

    pub fn relink_missing_media(
        &self,
        media_id: i64,
        old_path_missing: bool,
        filename: &str,
        full_path: &str,
        media_type: &str,
        file_size: i64,
        modified_at: i64,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            if !old_path_missing {
                return Err(rusqlite::Error::InvalidParameterName("Record is not missing".into()));
            }
            let existing: Option<i64> = connection.query_row("SELECT id FROM media WHERE full_path=?1", [full_path], |row| row.get(0)).ok();
            if let Some(other) = existing.filter(|other| *other != media_id) {
                let annotated: i64 = connection.query_row("SELECT (SELECT count(*) FROM media_tags WHERE media_id=?1) + (SELECT COALESCE(rating,0) FROM media WHERE id=?1)", [other], |row| row.get(0))?;
                if annotated > 0 {
                    return Err(rusqlite::Error::InvalidParameterName("Target already has annotations; choose another file".into()));
                }
                connection.execute("DELETE FROM media WHERE id=?1", [other])?;
            }
            queries::update_media_path(connection, media_id, filename, full_path, None)?;
            connection.execute("UPDATE media SET file_size=?1,modified_at=?2,thumb_path=NULL,micro_thumb=NULL,width=NULL,height=NULL,media_type=?4 WHERE id=?3", rusqlite::params![file_size,modified_at,media_id,media_type])?;
            Ok(())
        })
    }

    pub fn upsert_scanned_files(
        &self,
        folder_id: i64,
        files: &[ScanFileRecord],
    ) -> Result<Vec<Option<String>>, DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute_batch("BEGIN")?;
            let result = (|| {
                files
                    .iter()
                    .map(|file| {
                        queries::scan_upsert_media(
                            connection,
                            folder_id,
                            &file.filename,
                            &file.full_path,
                            &file.media_type,
                            file.file_size,
                            file.modified_at,
                            file.poster_path.as_deref(),
                        )
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()
            })();
            match result {
                Ok(result) => {
                    connection.execute_batch("COMMIT")?;
                    Ok(result)
                }
                Err(error) => {
                    let _ = connection.execute_batch("ROLLBACK");
                    Err(error)
                }
            }
        })
    }

    pub fn update_scanned_thumbnails(
        &self,
        updates: &[ThumbnailRecord],
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute_batch("BEGIN")?;
            let result = (|| {
                let mut statement = connection.prepare_cached("UPDATE media SET thumb_path=?1,width=COALESCE(?2,width),height=COALESCE(?3,height),micro_thumb=COALESCE(?4,micro_thumb) WHERE full_path=?5")?;
                for item in updates {
                    statement.execute(rusqlite::params![item.thumb_path,item.width,item.height,item.micro_thumb,item.full_path])?;
                }
                Ok::<_,rusqlite::Error>(())
            })();
            match result {
                Ok(()) => connection.execute_batch("COMMIT"),
                Err(error) => { let _ = connection.execute_batch("ROLLBACK"); Err(error) }
            }
        })
    }

    pub fn update_video_thumbnails(
        &self,
        updates: &[(String, String, Option<Vec<u8>>)],
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute_batch("BEGIN")?;
            let result = (|| {
                let mut statement = connection.prepare_cached(
                    "UPDATE media SET thumb_path=?1,micro_thumb=?2 WHERE full_path=?3",
                )?;
                for (full_path, thumb_path, micro) in updates {
                    statement.execute(rusqlite::params![
                        thumb_path,
                        micro.as_deref(),
                        full_path
                    ])?;
                }
                Ok::<_, rusqlite::Error>(())
            })();
            match result {
                Ok(()) => connection.execute_batch("COMMIT"),
                Err(error) => {
                    let _ = connection.execute_batch("ROLLBACK");
                    Err(error)
                }
            }
        })
    }

    pub fn update_backfilled_thumbnails(
        &self,
        updates: &[ThumbnailRecord],
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute_batch("BEGIN")?;
            let result = (|| {
                let mut statement = connection.prepare_cached("UPDATE media SET thumb_path=?1,width=?2,height=?3,micro_thumb=?4 WHERE full_path=?5")?;
                for item in updates {
                    statement.execute(rusqlite::params![item.thumb_path,item.width,item.height,item.micro_thumb,item.full_path])?;
                }
                Ok::<_,rusqlite::Error>(())
            })();
            match result {
                Ok(()) => connection.execute_batch("COMMIT"),
                Err(error) => { let _ = connection.execute_batch("ROLLBACK"); Err(error) }
            }
        })
    }

    pub fn media_paths_for_folder(
        &self,
        folder_id: i64,
    ) -> Result<Vec<(i64, String)>, DatabaseAccessError> {
        self.with_reader(|connection| {
            let mut statement =
                connection.prepare("SELECT id,full_path FROM media WHERE folder_id=?1")?;
            statement
                .query_map([folder_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect()
        })
    }

    pub fn delete_media_for_folder(
        &self,
        folder_id: i64,
        media_ids: &[i64],
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            let mut statement =
                connection.prepare_cached("DELETE FROM media WHERE id=?1 AND folder_id=?2")?;
            for id in media_ids {
                statement.execute(rusqlite::params![id, folder_id])?;
            }
            Ok(())
        })
    }

    pub fn reconcile_watched_files(
        &self,
        removed_paths: &[String],
        added_files: &[WatchedFile],
    ) -> Result<Vec<WatchedFile>, DatabaseAccessError> {
        self.with_writer(|connection| {
            for path in removed_paths {
                connection.execute("DELETE FROM media WHERE full_path=?1", [path])?;
            }
            let mut statement = connection.prepare("SELECT id,path FROM folders")?;
            let folders = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut indexed = Vec::new();
            for file in added_files {
                let path = std::path::Path::new(&file.path);
                let Some((folder_id, _)) = folders
                    .iter()
                    .filter(|(_, folder)| path.starts_with(folder))
                    .max_by_key(|(_, folder)| folder.len())
                else {
                    continue;
                };
                queries::watcher_upsert_media(
                    connection,
                    *folder_id,
                    &file.filename,
                    &file.path,
                    &file.media_type,
                    file.file_size,
                    file.modified_at,
                )?;
                indexed.push(file.clone());
            }
            Ok(indexed)
        })
    }

    pub fn reconcile_browser_files(
        &self,
        added_files: &[BrowserFile],
        removed_paths: &[String],
    ) -> Result<bool, DatabaseAccessError> {
        self.with_cache_writer(|connection| {
            let mut unknown_removal = false;
            for path in removed_paths {
                let known: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM media WHERE full_path=?1)",
                    [path],
                    |row| row.get(0),
                )?;
                unknown_removal |= !known;
                connection.execute("DELETE FROM live_paths WHERE full_path=?1", [path])?;
            }
            for file in added_files {
                crate::search_cache::cache_file(connection, file)?;
                connection.execute("INSERT OR IGNORE INTO live_paths VALUES(?1)", [&file.path])?;
            }
            Ok(unknown_removal)
        })
    }

    pub fn folder_paths(&self) -> Result<Vec<String>, DatabaseAccessError> {
        self.with_reader(|connection| {
            let mut statement = connection.prepare("SELECT path FROM folders WHERE path<>''")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect()
        })
    }

    pub fn gallery_snapshot(
        &self,
        query: &MediaQuery,
        directory_paths: &[String],
        grouping_enabled: bool,
        browser_root: Option<&str>,
        suppress_skeletons: bool,
        requested_offset: usize,
        window_size: usize,
        arrange: impl FnOnce(&mut Vec<MediaSkeleton>, &[MediaGroupingFields]) -> Vec<Option<String>>,
    ) -> Result<GalleryDatabaseSnapshot, DatabaseAccessError> {
        self.with_reader(|connection| {
            let folders = queries::list_folders(connection)?;
            let mut directory_counts = std::collections::HashMap::new();
            for path in directory_paths {
                directory_counts.insert(
                    path.clone(),
                    queries::count_media_under_path(connection, path)?,
                );
            }
            let mut skeletons = if suppress_skeletons {
                Vec::new()
            } else {
                queries::get_media_skeletons(connection, query)?
            };
            let mut fields = if grouping_enabled {
                queries::get_media_grouping_fields(connection, query)?
            } else {
                Vec::new()
            };
            if self.browser {
                for field in &mut fields {
                    field.root_path = browser_root.map(str::to_owned).unwrap_or_else(|| {
                        std::path::Path::new(&field.full_path)
                            .parent()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    });
                }
            }
            let group_labels = arrange(&mut skeletons, &fields);
            let media_count = skeletons.len() as i64;
            let window_offset = requested_offset.min(skeletons.len().saturating_sub(1));
            let media_ids = skeletons
                .iter()
                .skip(window_offset)
                .take(window_size)
                .map(|row| row.id)
                .collect::<Vec<_>>();
            let media = media_ids
                .iter()
                .map(|id| queries::get_media_by_id(connection, *id))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let media_tags = queries::get_tags_for_media_batch(connection, &ids)?;
            let available_extensions = queries::get_available_extensions(connection)?;
            let folder_counts = queries::count_media_per_folder(connection)?;
            let tag_tree = queries::get_tag_tree(connection)?;
            Ok(GalleryDatabaseSnapshot {
                folders,
                directory_counts,
                skeletons,
                group_labels,
                media_count,
                window_offset,
                media,
                media_tags,
                available_extensions,
                folder_counts,
                tag_tree,
            })
        })
    }

    pub fn get_media_by_ids(&self, ids: &[i64]) -> Result<Vec<MediaItem>, DatabaseAccessError> {
        self.with_reader(|connection| {
            ids.iter()
                .map(|id| queries::get_media_by_id(connection, *id))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map(|items| items.into_iter().flatten().collect())
    }

    pub fn get_media_page_with_tags(
        &self,
        query: &MediaQuery,
    ) -> Result<(Vec<MediaItem>, std::collections::HashMap<i64, Vec<Tag>>), DatabaseAccessError>
    {
        self.with_reader(|connection| {
            let media = queries::get_media(connection, query)?;
            let ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let tags = queries::get_tags_for_media_batch(connection, &ids)?;
            Ok((media, tags))
        })
    }

    pub fn get_media_window(
        &self,
        ids: &[i64],
    ) -> Result<(Vec<MediaItem>, std::collections::HashMap<i64, Vec<Tag>>), DatabaseAccessError>
    {
        self.with_reader(|connection| {
            let media = ids
                .iter()
                .map(|id| queries::get_media_by_id(connection, *id))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let ids = media.iter().map(|item| item.id).collect::<Vec<_>>();
            let tags = queries::get_tags_for_media_batch(connection, &ids)?;
            Ok((media, tags))
        })
    }

    pub fn media_paths_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<std::path::PathBuf>, DatabaseAccessError> {
        self.with_reader(|connection| {
            ids.iter()
                .map(|id| {
                    queries::get_media_by_id(connection, *id)
                        .map(|media| media.map(|item| std::path::PathBuf::from(item.full_path)))
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map(|items| items.into_iter().flatten().collect())
    }

    pub fn media_path_pairs_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<Vec<(std::path::PathBuf, i64)>, DatabaseAccessError> {
        self.with_reader(|connection| {
            ids.iter()
                .filter_map(|id| match queries::get_media_by_id(connection, *id) {
                    Ok(Some(media)) => Some(Ok((std::path::PathBuf::from(media.full_path), *id))),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect()
        })
    }

    pub fn ids_for_paths(&self, paths: &[String]) -> Result<Vec<i64>, DatabaseAccessError> {
        self.with_reader(|connection| {
            let mut ids = Vec::new();
            for path in paths {
                if let Ok(id) =
                    connection.query_row("SELECT id FROM media WHERE full_path=?1", [path], |row| {
                        row.get::<_, i64>(0)
                    })
                {
                    ids.push(id);
                }
            }
            Ok(ids)
        })
    }

    pub fn file_version_changed(
        &self,
        full_path: &str,
        version: &str,
    ) -> Result<bool, DatabaseAccessError> {
        self.with_reader(|connection| {
            let previous = connection
                .query_row(
                    "SELECT version FROM file_versions WHERE full_path=?1",
                    [full_path],
                    |row| row.get::<_, String>(0),
                )
                .ok();
            Ok(previous.as_deref() != Some(version))
        })
    }

    pub fn store_visible_thumbnail(
        &self,
        media_id: i64,
        full_path: &str,
        thumb_path: &str,
        width: Option<i64>,
        height: Option<i64>,
        micro_thumb: Option<&[u8]>,
        version: Option<&str>,
    ) -> Result<(), DatabaseAccessError> {
        self.with_cache_writer(|connection| {
            connection.execute("UPDATE media SET thumb_path=?1,width=?2,height=?3,micro_thumb=?4 WHERE id=?5 AND full_path=?6", rusqlite::params![thumb_path,width,height,micro_thumb,media_id,full_path])?;
            if let Some(version) = version {
                connection.execute("INSERT OR REPLACE INTO file_versions VALUES(?1,?2)", rusqlite::params![full_path,version])?;
            }
            Ok(())
        })
    }

    pub fn restore_record_preview(&self, item: &mut MediaItem) -> Result<(), DatabaseAccessError> {
        if !self.browser || !item.missing {
            return Ok(());
        }
        self.with_reader(|connection| {
            if let Ok(path) = connection.query_row(
                "SELECT thumb_path FROM tag_previews WHERE media_id=?1",
                [item.id],
                |row| row.get::<_, String>(0),
            ) {
                item.thumb_path = Some(path);
            }
            Ok(())
        })
    }

    pub fn restore_record_previews(
        &self,
        items: &mut [MediaItem],
    ) -> Result<(), DatabaseAccessError> {
        if !self.browser || !items.iter().any(|item| item.missing) {
            return Ok(());
        }
        self.with_reader(|connection| {
            let mut statement =
                connection.prepare("SELECT thumb_path FROM tag_previews WHERE media_id=?1")?;
            for item in items.iter_mut().filter(|item| item.missing) {
                if let Ok(path) = statement.query_row([item.id], |row| row.get::<_, String>(0)) {
                    item.thumb_path = Some(path);
                }
            }
            Ok(())
        })
    }

    pub fn open_count(&self, media_id: i64) -> i64 {
        self.with_reader(|connection| Ok(crate::get_open_count(connection, media_id)))
            .unwrap_or_default()
    }

    pub fn create_or_find_ai_tags(
        &self,
        labels: &[String],
    ) -> Result<Vec<i64>, DatabaseAccessError> {
        self.with_writer(|connection| {
            let parent = queries::find_or_create_tag(connection, "AI Suggested");
            let mut ids = std::collections::BTreeSet::new();
            for label in labels {
                if let Some(id) = queries::find_tag_by_name(connection, label) {
                    ids.insert(id);
                } else if let Ok(tag) = queries::create_tag(connection, label, parent) {
                    ids.insert(tag.id);
                }
            }
            Ok(ids.into_iter().collect())
        })
    }
    read_method!(list_folders() -> Vec<Folder>);
    read_method!(count_media_per_folder() -> std::collections::HashMap<i64, i64>);
    read_method!(count_media_under_path(prefix: &str) -> i64);
    write_method!(add_folder(path: &str) -> Folder);
    write_method!(remove_folder(folder_id: i64) -> ());
    write_method!(update_folder_scan_time(folder_id: i64) -> ());
    write_method!(insert_media(folder_id: i64, filename: &str, full_path: &str, media_type: &str, file_size: i64, modified_at: i64, width: Option<i64>, height: Option<i64>, thumb_path: Option<&str>) -> i64);
    write_method!(scan_upsert_media(folder_id: i64, filename: &str, full_path: &str, media_type: &str, file_size: i64, modified_at: i64, poster_path: Option<&str>) -> Option<String>);
    read_method!(get_media(query: &MediaQuery) -> Vec<MediaItem>);
    read_method!(get_media_skeletons(query: &MediaQuery) -> Vec<MediaSkeleton>);
    read_method!(get_media_grouping_fields(query: &MediaQuery) -> Vec<MediaGroupingFields>);
    read_method!(get_media_count(query: &MediaQuery) -> i64);
    read_method!(get_available_extensions() -> Vec<String>);
    write_method!(delete_media(media_id: i64) -> ());
    write_method!(update_media_path(media_id: i64, filename: &str, full_path: &str, folder_id: Option<i64>) -> ());
    read_method!(get_media_by_id(media_id: i64) -> Option<MediaItem>);
    write_method!(update_micro_thumb(full_path: &str, blob: &[u8]) -> ());
    read_method!(get_statistics() -> LibraryStats);
    read_method!(get_statistics_scoped(browser: bool) -> LibraryStats);
    read_method!(export_all_metadata() -> Vec<MediaExport>);
    read_method!(export_metadata_scoped(browser: bool) -> Vec<MediaExport>);
    pub fn find_tag_by_name(&self, name: &str) -> Result<Option<i64>, DatabaseAccessError> {
        self.with_reader(|connection| Ok(queries::find_tag_by_name(connection, name)))
    }
    pub fn find_or_create_tag(&self, name: &str) -> Result<Option<i64>, DatabaseAccessError> {
        self.with_writer(|connection| Ok(queries::find_or_create_tag(connection, name)))
    }
    write_method!(create_tag(name: &str, parent_id: Option<i64>) -> Tag);
    write_method!(update_tag_appearance(tag_id: i64, color: Option<&str>, icon: Option<&str>) -> ());
    write_method!(set_tag_display(tag_id: i64, display: bool) -> ());
    read_method!(get_tag_tree() -> Vec<Tag>);
    write_method!(move_tag(source: i64, drop: TagDropPos) -> Vec<i64>);
    write_method!(set_rating_bulk(media_ids: &[i64], rating: i32) -> ());
    write_method!(delete_tag(tag_id: i64) -> ());
    write_method!(rename_tag(tag_id: i64, new_name: &str) -> ());
    write_method!(merge_tag(source_id: i64, target_id: i64, reparent_children: bool) -> ());
    write_method!(watcher_upsert_media(folder_id: i64, filename: &str, full_path: &str, media_type: &str, file_size: i64, modified_at: i64) -> ());
    read_method!(get_ancestor_ids(tag_id: i64) -> Vec<i64>);
    write_method!(assign_tags(media_ids: &[i64], tag_ids: &[i64]) -> ());
    read_method!(get_tags_for_media_batch(media_ids: &[i64]) -> std::collections::HashMap<i64, Vec<Tag>>);
    write_method!(remove_tags(media_ids: &[i64], tag_ids: &[i64]) -> ());
    write_method!(remove_all_tags(media_ids: &[i64]) -> ());
    read_method!(get_associated_tags(tag_id: i64) -> Vec<i64>);
    write_method!(set_associated_tags(tag_id: i64, associated_ids: &[i64]) -> ());
    read_method!(expand_with_associated(tag_ids: &[i64]) -> Vec<i64>);
    read_method!(get_media_tags(media_id: i64) -> Vec<Tag>);
    read_method!(get_media_ids_with_tag(tag_id: i64) -> Vec<i64>);
    read_method!(get_paths_and_current_tags(media_ids: &[i64]) -> Vec<(String, Vec<String>)>);
}
