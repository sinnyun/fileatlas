use super::*;

/// Find a tag by name (case-insensitive). Returns the tag ID if found.
pub fn find_tag_by_name(conn: &Connection, name: &str) -> Option<i64> {
    conn.query_row(
        "SELECT id FROM tags WHERE LOWER(name) = LOWER(?1) LIMIT 1",
        params![name],
        |row| row.get(0),
    )
    .ok()
}

/// Find or create a top-level tag by name. Returns the tag ID.
pub fn find_or_create_tag(conn: &Connection, name: &str) -> Option<i64> {
    if let Some(id) = find_tag_by_name(conn, name) {
        return Some(id);
    }
    create_tag(conn, name, None).ok().map(|t| t.id)
}

pub fn export_all_metadata(conn: &Connection) -> rusqlite::Result<Vec<MediaExport>> {
    let mut media_stmt = conn.prepare(
        "SELECT id, filename, full_path, COALESCE(rating, 0) FROM media ORDER BY full_path",
    )?;
    let mut tag_stmt = conn.prepare(
        "SELECT t.full_path FROM tags t
         JOIN media_tags mt ON mt.tag_id = t.id
         WHERE mt.media_id = ?1
         ORDER BY t.full_path",
    )?;

    let rows = media_stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;

    let mut result = Vec::new();
    for row in rows {
        let (id, filename, full_path, rating) = row?;
        let tags: Vec<String> = tag_stmt
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .map(|fp| fp.replace('>', " > "))
            .collect();
        result.push(MediaExport {
            filename,
            full_path,
            rating,
            tags,
        });
    }
    Ok(result)
}
