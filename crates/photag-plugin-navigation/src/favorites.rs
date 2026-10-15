use super::*;
impl NavigationService {
    pub fn rename_favorite(&self, id: i64, name: &str) -> Result<(), NavigationError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NavigationError::NotDirectory(
                "Favorite name is empty".into(),
            ));
        }
        self.database.with_writer(|conn| {
            conn.execute("INSERT INTO favorite_meta(id,name,position) VALUES(?1,?2,?1) ON CONFLICT(id) DO UPDATE SET name=excluded.name",rusqlite::params![id,name])?;
            Ok(())
        })?;
        Ok(())
    }
    pub fn move_favorite(&self, id: i64, delta: isize) -> Result<(), NavigationError> {
        let mut folders = self.snapshot()?.folders;
        let Some(index) = folders.iter().position(|node| node.folder.id == id) else {
            return Ok(());
        };
        let target = index.saturating_add_signed(delta).min(folders.len() - 1);
        folders.swap(index, target);
        self.database.with_writer(|conn| {
            for (i,node) in folders.iter().enumerate() {
                conn.execute("INSERT INTO favorite_meta(id,name,position) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET position=excluded.position",rusqlite::params![node.folder.id,node.folder.name,i as i64])?;
            }
            Ok(())
        })?;
        Ok(())
    }
}
