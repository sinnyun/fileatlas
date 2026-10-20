//! Typed persistence for the clipboard workbench.
use crate::{DatabaseAccessError, DatabaseService};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClipboardCategoryRecord {
    pub id: i64,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn favorites_are_durable_and_only_explicit_removal_can_delete_them()
    -> Result<(), Box<dyn std::error::Error>> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fileatlas-favorites-{}-{stamp}.sqlite",
            std::process::id()
        ));
        let db = DatabaseService::open_browser_at(&path)?;
        // Simulate a pre-existing pinned row: no schema migration or data rewrite needed.
        let id = db.clipboard_add_manual("legacy pinned favorite", None, true, 1)?;
        assert!(db.clipboard_delete(id).is_err());
        db.clipboard_add_manual("legacy pinned favorite", None, false, 2)?;
        db.clipboard_update(id, "edited local favorite", None, false, 3)?;
        db.clipboard_capture("temporary", 4, 0)?;
        db.clipboard_clear_unpinned()?;
        let items = db.clipboard_snapshot()?.1;
        assert_eq!(items.len(), 1);
        assert!(items[0].pinned);
        assert_eq!(items[0].content, "edited local favorite");
        drop(db);
        let db = DatabaseService::open_browser_at(&path)?;
        assert_eq!(db.clipboard_snapshot()?.1, items);
        assert!(db.clipboard_delete(id).is_err());
        db.clipboard_remove_favorite(id)?;
        let id = db.clipboard_add_manual("new favorite", None, false, 5)?;
        assert!(db.clipboard_remove_favorite(id).is_err());
        db.clipboard_add_favorite(id)?;
        assert!(db.clipboard_snapshot()?.1[0].pinned);
        db.clipboard_remove_favorite(id)?;
        drop(db);
        let db = DatabaseService::open_browser_at(&path)?;
        assert!(db.clipboard_snapshot()?.1.is_empty());
        drop(db);
        let _ = std::fs::remove_file(path);
        Ok(())
    }
    #[test]
    fn clipboard_group_survives_recapture_reopen_and_removal()
    -> Result<(), Box<dyn std::error::Error>> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "fileatlas-clipboard-groups-{}-{stamp}.sqlite",
            std::process::id()
        ));
        let db = DatabaseService::open_browser_at(&path)?;
        db.clipboard_add_category("工作")?;
        db.clipboard_capture("persisted clipboard fixture", 100, 500)?;
        let (groups, items) = db.clipboard_snapshot()?;
        let (group, item) = (groups[0].id, items[0].id);
        db.clipboard_assign_category(item, Some(group))?;
        db.clipboard_capture("persisted clipboard fixture", 200, 500)?;
        let (_, items) = db.clipboard_snapshot()?;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category_id, Some(group));
        db.clipboard_rename_category(group, "项目")?;
        drop(db);
        let db = DatabaseService::open_browser_at(&path)?;
        let (groups, items) = db.clipboard_snapshot()?;
        assert_eq!(
            groups.len(),
            1,
            "browser mode must persist clipboard groups across process restarts"
        );
        assert_eq!(
            items.len(),
            1,
            "browser mode must persist clipboard history across process restarts"
        );
        assert_eq!(groups[0].name, "项目");
        assert_eq!(items[0].category_id, Some(group));
        db.clipboard_delete_category(group)?;
        let (groups, items) = db.clipboard_snapshot()?;
        assert!(groups.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category_id, None);
        drop(db);
        let db = DatabaseService::open_browser_at(&path)?;
        let (groups, items) = db.clipboard_snapshot()?;
        assert!(
            groups.is_empty(),
            "deleted groups must not reappear after restart"
        );
        assert_eq!(items[0].category_id, None);
        db.clipboard_delete(item)?;
        drop(db);
        let db = DatabaseService::open_browser_at(&path)?;
        assert!(
            db.clipboard_snapshot()?.1.is_empty(),
            "deleted history must not reappear after restart"
        );
        drop(db);
        let _ = std::fs::remove_file(path);
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClipboardHistoryRecord {
    pub id: i64,
    pub content: String,
    pub content_type: String,
    pub image_path: Option<String>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub category_id: Option<i64>,
    /// Durable favorite flag; stored in the legacy `pinned` column for compatibility.
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

impl DatabaseService {
    pub fn clipboard_add_manual(
        &self,
        content: &str,
        category: Option<i64>,
        pinned: bool,
        now: i64,
    ) -> Result<i64, DatabaseAccessError> {
        self.with_writer(|c|{c.execute("INSERT INTO clipboard_history(content,category_id,pinned,created_at,last_used_at) VALUES(?1,?2,?3,?4,?4) ON CONFLICT(content) DO UPDATE SET category_id=excluded.category_id,pinned=MAX(clipboard_history.pinned,excluded.pinned),last_used_at=excluded.last_used_at",params![content,category,i64::from(pinned),now])?;let id=c.query_row("SELECT id FROM clipboard_history WHERE content=?1",[content],|r|r.get(0))?;Ok(id)})
    }
    pub fn clipboard_snapshot(
        &self,
    ) -> Result<(Vec<ClipboardCategoryRecord>, Vec<ClipboardHistoryRecord>), DatabaseAccessError>
    {
        self.with_reader(|c| {
            let categories = c.prepare("SELECT id,name FROM clipboard_categories ORDER BY position,name")?
                .query_map([], |r| Ok(ClipboardCategoryRecord { id:r.get(0)?, name:r.get(1)? }))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let items = c.prepare("SELECT id,content,content_type,image_path,image_width,image_height,category_id,pinned,created_at,last_used_at FROM clipboard_history ORDER BY pinned DESC,last_used_at DESC")?
                .query_map([], |r| Ok(ClipboardHistoryRecord { id:r.get(0)?,content:r.get(1)?,content_type:r.get(2)?,image_path:r.get(3)?,image_width:r.get(4)?,image_height:r.get(5)?,category_id:r.get(6)?,pinned:r.get::<_,i64>(7)?!=0,created_at:r.get(8)?,last_used_at:r.get(9)? }))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((categories,items))
        })
    }
    pub fn clipboard_capture(
        &self,
        content: &str,
        now: i64,
        limit: i64,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            c.execute("INSERT INTO clipboard_history(content,created_at,last_used_at) VALUES(?1,?2,?2) ON CONFLICT(content) DO UPDATE SET last_used_at=excluded.last_used_at", params![content,now])?;
            c.execute("DELETE FROM clipboard_history WHERE pinned=0 AND id NOT IN (SELECT id FROM clipboard_history WHERE pinned=0 ORDER BY last_used_at DESC LIMIT ?1)", [limit])?;
            Ok(())
        })
    }

    pub fn clipboard_capture_image(
        &self,
        hash: &str,
        image_path: &str,
        width: u32,
        height: u32,
        now: i64,
        limit: i64,
    ) -> Result<(), DatabaseAccessError> {
        let content = format!("image:{hash}");
        self.with_writer(|c| {
            c.execute(
                "INSERT INTO clipboard_history(content,content_type,image_path,image_width,image_height,created_at,last_used_at) VALUES(?1,'image',?2,?3,?4,?5,?5) ON CONFLICT(content) DO UPDATE SET content_type='image',image_path=excluded.image_path,image_width=excluded.image_width,image_height=excluded.image_height,last_used_at=excluded.last_used_at",
                params![content, image_path, width, height, now],
            )?;
            c.execute(
                "DELETE FROM clipboard_history WHERE pinned=0 AND id NOT IN (SELECT id FROM clipboard_history WHERE pinned=0 ORDER BY last_used_at DESC LIMIT ?1)",
                [limit],
            )?;
            Ok(())
        })
    }
    pub fn clipboard_mark_used(&self, id: i64, now: i64) -> Result<String, DatabaseAccessError> {
        self.with_writer(|c| {
            let text = c.query_row(
                "SELECT content FROM clipboard_history WHERE id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )?;
            c.execute(
                "UPDATE clipboard_history SET last_used_at=?2 WHERE id=?1",
                params![id, now],
            )?;
            Ok(text)
        })
    }
    pub fn clipboard_update(
        &self,
        id: i64,
        content: &str,
        category: Option<i64>,
        pinned: bool,
        now: i64,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| { c.execute("UPDATE clipboard_history SET content=?2,category_id=?3,pinned=MAX(pinned,?4),last_used_at=?5 WHERE id=?1",params![id,content,category,i64::from(pinned),now])?; Ok(()) })
    }
    /// The legacy SQLite `pinned` column now represents durable favorites.
    pub fn clipboard_add_favorite(&self, id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            if c.execute("UPDATE clipboard_history SET pinned=1 WHERE id=?1", [id])? == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
    /// Only this explicit operation may delete locally saved favorite content.
    pub fn clipboard_remove_favorite(&self, id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            if c.execute(
                "DELETE FROM clipboard_history WHERE id=?1 AND pinned<>0",
                [id],
            )? == 0
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
    pub fn clipboard_assign_category(
        &self,
        id: i64,
        category: Option<i64>,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            let changed = c.execute(
                "UPDATE clipboard_history SET category_id=?2 WHERE id=?1",
                params![id, category],
            )?;
            if changed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }
    pub fn clipboard_add_category(&self, name: &str) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| { c.execute("INSERT OR IGNORE INTO clipboard_categories(name,position) SELECT ?1,COALESCE(MAX(position),-1)+1 FROM clipboard_categories",[name])?; Ok(()) })
    }
    pub fn clipboard_rename_category(
        &self,
        id: i64,
        name: &str,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            c.execute(
                "UPDATE clipboard_categories SET name=?2 WHERE id=?1",
                params![id, name],
            )?;
            Ok(())
        })
    }
    pub fn clipboard_delete_category(&self, id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            // Keep history even on a connection that does not enforce foreign keys.
            let tx = c.unchecked_transaction()?;
            tx.execute(
                "UPDATE clipboard_history SET category_id=NULL WHERE category_id=?1",
                [id],
            )?;
            tx.execute("DELETE FROM clipboard_categories WHERE id=?1", [id])?;
            tx.commit()?;
            Ok(())
        })
    }
    pub fn clipboard_clear_unpinned(&self) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            c.execute("DELETE FROM clipboard_history WHERE pinned=0", [])?;
            Ok(())
        })
    }
    pub fn clipboard_delete(&self, id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|c| {
            if c.execute(
                "DELETE FROM clipboard_history WHERE id=?1 AND pinned=0",
                [id],
            )? == 0
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "收藏内容请使用‘从收藏中删除’".into(),
                ));
            }
            Ok(())
        })
    }
}
