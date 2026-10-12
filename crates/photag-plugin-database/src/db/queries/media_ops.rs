use super::*;

/// Delete a media row by id. Cascade rules in the schema take care of
/// `media_tags` rows. Caller is responsible for deleting the file/thumbnail
/// on disk.
pub fn delete_media(conn: &Connection, media_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media WHERE id = ?1", params![media_id])?;
    Ok(())
}

/// Update both filename and full_path for a media row. Used after rename / move.
pub fn update_media_path(
    conn: &Connection,
    media_id: i64,
    new_filename: &str,
    new_full_path: &str,
    new_folder_id: Option<i64>,
) -> rusqlite::Result<()> {
    if let Some(fid) = new_folder_id {
        conn.execute(
            "UPDATE media SET filename = ?1, full_path = ?2, folder_id = ?3 WHERE id = ?4",
            params![new_filename, new_full_path, fid, media_id],
        )?;
    } else {
        conn.execute(
            "UPDATE media SET filename = ?1, full_path = ?2 WHERE id = ?3",
            params![new_filename, new_full_path, media_id],
        )?;
    }
    Ok(())
}

/// Look up a media item by id (used by the Delete/Copy/Move pending actions
/// when only an id is held).
pub fn get_media_by_id(conn: &Connection, media_id: i64) -> rusqlite::Result<Option<MediaItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, folder_id, filename, full_path, media_type, file_size, modified_at,
                width, height, date_taken, thumb_path, micro_thumb,
                COALESCE(rating, 0)
           FROM media WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![media_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(MediaItem {
            id: row.get(0)?,
            folder_id: row.get(1)?,
            filename: row.get(2)?,
            full_path: row.get(3)?,
            media_type: row.get(4)?,
            file_size: row.get(5)?,
            modified_at: row.get(6)?,
            width: row.get(7)?,
            height: row.get(8)?,
            date_taken: row.get(9)?,
            thumb_path: row.get(10)?,
            micro_thumb: row.get(11)?,
            rating: row.get::<_, i64>(12).unwrap_or(0) as i32,
        }))
    } else {
        Ok(None)
    }
}

/// Store a tiny micro-thumbnail BLOB for instant placeholders.
pub fn update_micro_thumb(conn: &Connection, full_path: &str, blob: &[u8]) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE media SET micro_thumb = ?1 WHERE full_path = ?2",
        params![blob, full_path],
    )?;
    Ok(())
}

// ── Statistics ────────────────────────────────────────────────────────────

pub fn get_statistics(conn: &Connection) -> rusqlite::Result<LibraryStats> {
    let total_photos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM media WHERE media_type = 'image'",
        [],
        |r| r.get(0),
    )?;
    let total_videos: i64 = conn.query_row(
        "SELECT COUNT(*) FROM media WHERE media_type = 'video'",
        [],
        |r| r.get(0),
    )?;
    let total_size_bytes: u64 =
        conn.query_row("SELECT COALESCE(SUM(file_size), 0) FROM media", [], |r| {
            r.get::<_, i64>(0).map(|v| v as u64)
        })?;

    // Rating distribution
    let mut rating_counts = [0i64; 6];
    {
        let mut stmt =
            conn.prepare("SELECT COALESCE(rating, 0) as r, COUNT(*) FROM media GROUP BY r")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
        for r in rows {
            if let Ok((rating, count)) = r {
                let idx = (rating as usize).min(5);
                rating_counts[idx] = count;
            }
        }
    }

    // Top 10 tags by assignment count
    let top_tags = {
        let mut stmt = conn.prepare(
            "SELECT t.name, COUNT(mt.media_id) as cnt
             FROM tags t JOIN media_tags mt ON mt.tag_id = t.id
             GROUP BY t.id ORDER BY cnt DESC LIMIT 10",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    // Photos per month (last 12 months)
    let monthly_counts = {
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m', datetime(COALESCE(date_taken, modified_at), 'unixepoch')) as month,
                    COUNT(*)
             FROM media
             GROUP BY month
             ORDER BY month DESC
             LIMIT 12"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(LibraryStats {
        total_photos,
        total_videos,
        total_size_bytes,
        rating_counts,
        top_tags,
        monthly_counts,
    })
}

// ── Metadata Export ───────────────────────────────────────────────────────
