use super::*;

/// Used by the file-system watcher: insert new media OR update timestamps for
/// existing entries without ever replacing the row (which would cascade-delete
/// media_tags and lose all tag assignments).
pub fn watcher_upsert_media(
    conn: &Connection,
    folder_id: i64,
    filename: &str,
    full_path: &str,
    media_type: &str,
    file_size: i64,
    modified_at: i64,
) -> rusqlite::Result<()> {
    // ON CONFLICT: refresh file_size only. We deliberately do NOT touch
    // `modified_at` for existing rows, so an XMP/IPTC tag write (which
    // bumps the file's mtime) doesn't appear to the user as a new
    // creation date. The first scan still records mtime for fresh files.
    conn.execute(
        "INSERT INTO media (folder_id, filename, full_path, media_type, file_size, modified_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(full_path) DO UPDATE SET
             file_size    = excluded.file_size",
        params![
            folder_id,
            filename,
            full_path,
            media_type,
            file_size,
            modified_at
        ],
    )?;
    Ok(())
}

/// Return the IDs of every strict ancestor of `tag_id` (not including itself).
pub fn get_ancestor_ids(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut result = Vec::new();
    let mut stmt = conn.prepare("SELECT parent_id FROM tags WHERE id = ?1")?;
    let mut current: Option<i64> = stmt
        .query_row(params![tag_id], |row| row.get::<_, Option<i64>>(0))
        .ok()
        .flatten();
    while let Some(pid) = current {
        result.push(pid);
        current = stmt
            .query_row(params![pid], |row| row.get::<_, Option<i64>>(0))
            .ok()
            .flatten();
    }
    Ok(result)
}

/// Walk each tag's parent chain and return the union of all tag IDs + ancestors.
/// Used so that assigning a child tag implicitly assigns its parents too.
fn expand_with_ancestors(conn: &Connection, tag_ids: &[i64]) -> rusqlite::Result<Vec<i64>> {
    let mut result: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut stmt = conn.prepare("SELECT parent_id FROM tags WHERE id = ?1")?;
    for &id in tag_ids {
        let mut current = Some(id);
        while let Some(cid) = current {
            if !result.insert(cid) {
                break;
            } // already visited (cycle / shared ancestor)
            current = stmt
                .query_row(params![cid], |row| row.get::<_, Option<i64>>(0))
                .ok()
                .flatten();
        }
    }
    Ok(result.into_iter().collect())
}

pub fn assign_tags(conn: &Connection, media_ids: &[i64], tag_ids: &[i64]) -> rusqlite::Result<()> {
    let with_associated = expand_with_associated(conn, tag_ids)?;
    let expanded = expand_with_ancestors(conn, &with_associated)?;
    let manual: std::collections::HashSet<i64> = with_associated.iter().copied().collect();
    let tx = conn.unchecked_transaction()?;
    // For each (media, tag): insert if absent with the correct auto flag.
    // If already present, only upgrade auto→manual (manual→auto is never
    // a downgrade we want — a manual assignment is sticky).
    let mut stmt = tx.prepare(
        "INSERT INTO media_tags (media_id, tag_id, auto_applied) VALUES (?1, ?2, ?3) \
         ON CONFLICT(media_id, tag_id) DO UPDATE SET \
             auto_applied = CASE WHEN excluded.auto_applied = 0 THEN 0 \
                                 ELSE auto_applied END",
    )?;
    for &mid in media_ids {
        for &tid in &expanded {
            let flag: i64 = if manual.contains(&tid) { 0 } else { 1 };
            stmt.execute(params![mid, tid, flag])?;
        }
    }
    drop(stmt);
    tx.commit()
}

/// Bulk-fetch tag assignments for many media items at once.
/// Chunked to stay under SQLite's ~999-parameter limit.
pub fn get_tags_for_media_batch(
    conn: &Connection,
    media_ids: &[i64],
) -> rusqlite::Result<std::collections::HashMap<i64, Vec<Tag>>> {
    let mut result: std::collections::HashMap<i64, Vec<Tag>> = std::collections::HashMap::new();
    if media_ids.is_empty() {
        return Ok(result);
    }

    for chunk in media_ids.chunks(500) {
        let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{}", i)).collect();
        let sql = format!(
            "SELECT mt.media_id, t.id, t.name, t.parent_id, t.full_path, t.color, t.icon, \
                    COALESCE(t.display, 1), COALESCE(t.sort_order, 0) \
             FROM media_tags mt JOIN tags t ON mt.tag_id = t.id \
             WHERE mt.media_id IN ({}) ORDER BY t.full_path",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = chunk
            .iter()
            .map(|i| i as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let media_id: i64 = row.get(0)?;
            let tag = Tag {
                id: row.get(1)?,
                name: row.get(2)?,
                parent_id: row.get(3)?,
                full_path: row.get(4)?,
                children: vec![],
                media_count: 0,
                color: row.get(5)?,
                icon: row.get(6)?,
                display: row.get::<_, i64>(7)? != 0,
                sort_order: row.get(8)?,
            };
            Ok((media_id, tag))
        })?;
        for r in rows {
            let (mid, tag) = r?;
            result.entry(mid).or_default().push(tag);
        }
    }
    Ok(result)
}

pub fn remove_tags(conn: &Connection, media_ids: &[i64], tag_ids: &[i64]) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("DELETE FROM media_tags WHERE media_id = ?1 AND tag_id = ?2")?;
    for &mid in media_ids {
        for &tid in tag_ids {
            stmt.execute(params![mid, tid])?;
        }
    }
    Ok(())
}

/// Remove every tag assignment from the given media items in one shot.
/// Used by the preview toolbar's "Clear all tags" action.
pub fn remove_all_tags(conn: &Connection, media_ids: &[i64]) -> rusqlite::Result<()> {
    if media_ids.is_empty() {
        return Ok(());
    }
    let mut stmt = conn.prepare("DELETE FROM media_tags WHERE media_id = ?1")?;
    for &mid in media_ids {
        stmt.execute(params![mid])?;
    }
    Ok(())
}

/// Get the IDs of all tags associated with the given tag.
pub fn get_associated_tags(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT associated_tag_id FROM tag_associations WHERE tag_id = ?1")?;
    let rows = stmt.query_map(params![tag_id], |row| row.get(0))?;
    rows.collect()
}

/// Replace the associated tags for `tag_id` with `associated_ids`.
pub fn set_associated_tags(
    conn: &Connection,
    tag_id: i64,
    associated_ids: &[i64],
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM tag_associations WHERE tag_id = ?1",
        params![tag_id],
    )?;
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO tag_associations (tag_id, associated_tag_id) VALUES (?1, ?2)",
    )?;
    for &aid in associated_ids {
        stmt.execute(params![tag_id, aid])?;
    }
    Ok(())
}

/// Expand a set of tag IDs by adding each tag's associated tags (non-recursive,
/// one level only, one-directional).
pub fn expand_with_associated(conn: &Connection, tag_ids: &[i64]) -> rusqlite::Result<Vec<i64>> {
    let mut result: std::collections::HashSet<i64> = tag_ids.iter().copied().collect();
    let mut stmt =
        conn.prepare("SELECT associated_tag_id FROM tag_associations WHERE tag_id = ?1")?;
    for &id in tag_ids {
        let rows = stmt.query_map(params![id], |row| row.get::<_, i64>(0))?;
        for aid in rows {
            result.insert(aid?);
        }
    }
    Ok(result.into_iter().collect())
}

pub fn get_media_tags(conn: &Connection, media_id: i64) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.parent_id, t.full_path, t.color, t.icon, \
                COALESCE(t.display, 1), COALESCE(t.sort_order, 0) \
         FROM tags t JOIN media_tags mt ON t.id = mt.tag_id \
         WHERE mt.media_id = ?1 ORDER BY t.full_path",
    )?;
    let rows = stmt.query_map(params![media_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            parent_id: row.get(2)?,
            full_path: row.get(3)?,
            children: vec![],
            media_count: 0,
            color: row.get(4)?,
            icon: row.get(5)?,
            display: row.get::<_, i64>(6)? != 0,
            sort_order: row.get(7)?,
        })
    })?;
    rows.collect()
}

/// Return the IDs of every media item that has the given tag assigned.
/// Call this *before* deleting the tag so the cascade hasn't happened yet.
pub fn get_media_ids_with_tag(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare("SELECT media_id FROM media_tags WHERE tag_id = ?1")?;
    let rows = stmt.query_map(params![tag_id], |row| row.get(0))?;
    rows.collect()
}

/// For each media ID, return its file path and the full_path of every tag
/// currently assigned to it.  Used to build XMP sidecars after a tag change.
pub fn get_paths_and_current_tags(
    conn: &Connection,
    media_ids: &[i64],
) -> rusqlite::Result<Vec<(String, Vec<String>)>> {
    let mut result = Vec::new();
    let mut path_stmt = conn.prepare("SELECT full_path FROM media WHERE id = ?1")?;
    let mut tag_stmt = conn.prepare(
        "SELECT t.full_path FROM tags t \
         JOIN media_tags mt ON t.id = mt.tag_id \
         WHERE mt.media_id = ?1 ORDER BY t.full_path",
    )?;
    for &mid in media_ids {
        let full_path: String = path_stmt.query_row(params![mid], |row| row.get(0))?;
        let tags: Vec<String> = tag_stmt
            .query_map(params![mid], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        result.push((full_path, tags));
    }
    Ok(result)
}
