//! Typed database operations for the program launcher feature.

use crate::{DatabaseAccessError, DatabaseService};
use rusqlite::params;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherCategoryRecord {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherEntryRecord {
    pub id: i64,
    pub discovery_key: String,
    pub display_name: String,
    pub launch_path: String,
    pub uninstall_command: Option<String>,
    pub category_id: i64,
    pub hidden: bool,
    pub source: String,
    pub installed_on: Option<String>,
    pub size_bytes: Option<i64>,
    pub pin_scope: i64,
    pub custom_name: bool,
    pub custom_category: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherDiscoveredProgram {
    pub discovery_key: String,
    pub display_name: String,
    pub launch_path: String,
    pub uninstall_command: Option<String>,
    pub category_name: String,
    pub source: String,
    pub installed_on: Option<String>,
    pub size_bytes: Option<i64>,
}

impl DatabaseService {
    pub fn launcher_ensure_default_category(&self) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute(
                "INSERT OR IGNORE INTO app_launcher_categories(name, position) VALUES ('General', 0)",
                [],
            )?;
            Ok(())
        })
    }

    pub fn launcher_snapshot(
        &self,
    ) -> Result<(Vec<LauncherCategoryRecord>, Vec<LauncherEntryRecord>), DatabaseAccessError> {
        self.with_reader(|connection| {
            let categories = {
                let mut statement = connection.prepare(
                    "SELECT id, name, position FROM app_launcher_categories ORDER BY position, name",
                )?;
                statement
                    .query_map([], |row| {
                        Ok(LauncherCategoryRecord {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            position: row.get(2)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            let entries = {
                let mut statement = connection.prepare(
                    "SELECT e.id, e.display_name, e.launch_path, e.uninstall_command, e.category_id, e.hidden, e.source, m.installed_on, m.size_bytes, COALESCE(m.pin_scope, 0), e.custom_name, e.custom_category, e.discovery_key
                     FROM app_launcher_entries e LEFT JOIN app_launcher_metadata m ON m.entry_id=e.id ORDER BY e.display_name COLLATE NOCASE",
                )?;
                statement
                    .query_map([], |row| {
                        Ok(LauncherEntryRecord {
                            id: row.get(0)?,
                            display_name: row.get(1)?,
                            launch_path: row.get(2)?,
                            uninstall_command: row.get(3)?,
                            category_id: row.get(4)?,
                            hidden: row.get::<_, i64>(5)? != 0,
                            source: row.get(6)?,
                            installed_on: row.get(7)?,
                            size_bytes: row.get(8)?,
                            pin_scope: row.get(9)?,
                            custom_name: row.get::<_, i64>(10)? != 0,
                            custom_category: row.get::<_, i64>(11)? != 0,
                            discovery_key: row.get(12)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?
            };
            Ok((categories, entries))
        })
    }

    pub fn launcher_merge_discovered(
        &self,
        programs: &[LauncherDiscoveredProgram],
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            for program in programs {
                connection.execute(
                    "INSERT OR IGNORE INTO app_launcher_categories(name, position)
                     SELECT ?1, COALESCE(MAX(position), -1) + 1 FROM app_launcher_categories",
                    [&program.category_name],
                )?;
                let category_id: i64 = connection.query_row(
                    "SELECT id FROM app_launcher_categories WHERE name=?1",
                    [&program.category_name],
                    |row| row.get(0),
                )?;
                connection.execute(
                    "INSERT INTO app_launcher_entries
                       (discovery_key, display_name, launch_path, uninstall_command, category_id, source)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(discovery_key) DO UPDATE SET
                       display_name=CASE WHEN custom_name=0 THEN excluded.display_name ELSE display_name END,
                       launch_path=excluded.launch_path,
                       uninstall_command=COALESCE(excluded.uninstall_command, uninstall_command),
                       category_id=CASE WHEN custom_category=0 THEN excluded.category_id ELSE category_id END,
                       source=excluded.source",
                    params![
                        program.discovery_key,
                        program.display_name,
                        program.launch_path,
                        program.uninstall_command,
                        category_id,
                        program.source,
                    ],
                )?;
                connection.execute(
                    "INSERT INTO app_launcher_metadata(entry_id, installed_on, size_bytes)
                     SELECT id, ?2, ?3 FROM app_launcher_entries WHERE discovery_key=?1
                     ON CONFLICT(entry_id) DO UPDATE SET installed_on=excluded.installed_on, size_bytes=excluded.size_bytes",
                    params![program.discovery_key, program.installed_on, program.size_bytes],
                )?;
            }
            Ok(())
        })
    }

    pub fn launcher_add_category(&self, name: &str) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            let position: i64 = connection.query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM app_launcher_categories",
                [],
                |row| row.get(0),
            )?;
            connection.execute(
                "INSERT INTO app_launcher_categories(name, position) VALUES (?1, ?2)",
                params![name, position],
            )?;
            Ok(())
        })
    }

    pub fn launcher_rename_category(&self, id: i64, name: &str) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute(
                "UPDATE app_launcher_categories SET name=?1 WHERE id=?2",
                params![name, id],
            )?;
            Ok(())
        })
    }

    pub fn launcher_delete_category(&self, id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            let general_id: i64 = connection.query_row(
                "SELECT id FROM app_launcher_categories WHERE name='General'",
                [],
                |row| row.get(0),
            )?;
            if general_id == id {
                return Err(rusqlite::Error::InvalidParameterName(
                    "the General category cannot be removed".into(),
                ));
            }
            let transaction = connection.unchecked_transaction()?;
            transaction.execute(
                "UPDATE app_launcher_entries SET category_id=?1, custom_category=1 WHERE category_id=?2",
                params![general_id, id],
            )?;
            transaction.execute("DELETE FROM app_launcher_categories WHERE id=?1", [id])?;
            transaction.commit()
        })
    }

    pub fn launcher_update_entry(
        &self,
        id: i64,
        name: &str,
        category_id: i64,
        hidden: bool,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            let changed = connection.execute("UPDATE app_launcher_entries SET custom_name=CASE WHEN display_name<>?2 THEN 1 ELSE custom_name END, custom_category=CASE WHEN category_id<>?3 THEN 1 ELSE custom_category END, display_name=?2, category_id=?3, hidden=?4 WHERE id=?1 AND EXISTS(SELECT 1 FROM app_launcher_categories WHERE id=?3)", params![id, name, category_id, hidden])?;
            if changed == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
            Ok(())
        })
    }

    pub fn launcher_rename_entry(&self, id: i64, name: &str) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute(
                "UPDATE app_launcher_entries SET display_name=?1, custom_name=1 WHERE id=?2",
                params![name, id],
            )?;
            Ok(())
        })
    }

    pub fn launcher_move_entry(
        &self,
        id: i64,
        category_id: i64,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute(
                "UPDATE app_launcher_entries SET category_id=?1, custom_category=1 WHERE id=?2",
                params![category_id, id],
            )?;
            Ok(())
        })
    }

    pub fn launcher_set_hidden(&self, id: i64, hidden: bool) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute(
                "UPDATE app_launcher_entries SET hidden=?1 WHERE id=?2",
                params![if hidden { 1_i64 } else { 0_i64 }, id],
            )?;
            Ok(())
        })
    }

    pub fn launcher_set_pin_scope(
        &self,
        id: i64,
        pin_scope: i64,
    ) -> Result<(), DatabaseAccessError> {
        if !(0..=2).contains(&pin_scope) {
            return Err(rusqlite::Error::InvalidParameterName(
                "pin scope must be 0 (none), 1 (category), or 2 (all programs)".into(),
            )
            .into());
        }
        self.with_writer(|connection| {
            let changed = connection.execute(
                "INSERT INTO app_launcher_metadata(entry_id, pin_scope) VALUES (?1, ?2)
                 ON CONFLICT(entry_id) DO UPDATE SET pin_scope=excluded.pin_scope",
                params![id, pin_scope],
            )?;
            if changed == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
    }

    pub fn launcher_remove_entry(&self, id: i64) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            connection.execute("DELETE FROM app_launcher_entries WHERE id=?1", [id])?;
            Ok(())
        })
    }

    pub fn launcher_add_manual(
        &self,
        path: &Path,
        discovery_key: &str,
        display_name: &str,
    ) -> Result<(), DatabaseAccessError> {
        self.with_writer(|connection| {
            let category_id: i64 = connection.query_row(
                "SELECT id FROM app_launcher_categories WHERE name='General'",
                [],
                |row| row.get(0),
            )?;
            connection.execute(
                "INSERT INTO app_launcher_entries(discovery_key, display_name, launch_path, category_id, source)
                 VALUES (?1, ?2, ?3, ?4, 'manual')
                 ON CONFLICT(discovery_key) DO NOTHING",
                params![discovery_key, display_name, path.to_string_lossy(), category_id],
            )?;
            Ok(())
        })
    }
}
