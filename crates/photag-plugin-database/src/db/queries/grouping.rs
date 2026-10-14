//! Compact grouping inputs. No filesystem values are invented or persisted.
use super::*;
#[derive(Debug)]
pub struct MediaGroupingFields {
    pub id: i64,
    pub full_path: String,
    pub modified_at: i64,
    pub root_path: String,
}
pub fn get_media_grouping_fields(
    conn: &Connection,
    query: &MediaQuery,
) -> rusqlite::Result<Vec<MediaGroupingFields>> {
    let (joins, conditions, values) = build_filters(query);
    let mut sql = String::from(
        "SELECT DISTINCT m.id, m.full_path, m.modified_at, f.path FROM media m JOIN folders f ON f.id = m.folder_id",
    );
    sql.push_str(&joins);
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.prepare(&sql)?
        .query_map(params.as_slice(), |r| {
            Ok(MediaGroupingFields {
                id: r.get(0)?,
                full_path: r.get(1)?,
                modified_at: r.get(2)?,
                root_path: r.get(3)?,
            })
        })?
        .collect()
}
