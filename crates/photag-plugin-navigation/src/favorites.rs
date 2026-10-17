use super::*;
impl NavigationService {
    pub fn rename_favorite(&self, id: i64, name: &str) -> Result<(), NavigationError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(NavigationError::NotDirectory(
                "Favorite name is empty".into(),
            ));
        }
        self.database.set_favorite_name(id, name)?;
        Ok(())
    }
    pub fn move_favorite(&self, id: i64, delta: isize) -> Result<(), NavigationError> {
        let mut folders = self.snapshot()?.folders;
        let Some(index) = folders.iter().position(|node| node.folder.id == id) else {
            return Ok(());
        };
        let target = index.saturating_add_signed(delta).min(folders.len() - 1);
        folders.swap(index, target);
        let entries = folders
            .iter()
            .map(|node| (node.folder.id, node.folder.name.clone()))
            .collect::<Vec<_>>();
        self.database.set_favorite_order(&entries)?;
        Ok(())
    }
}
