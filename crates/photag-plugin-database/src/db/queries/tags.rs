use super::*;

pub fn create_tag(conn: &Connection, name: &str, parent_id: Option<i64>) -> rusqlite::Result<Tag> {
    let full_path = if let Some(pid) = parent_id {
        let parent_path: String = conn.query_row(
            "SELECT full_path FROM tags WHERE id = ?1",
            params![pid],
            |row| row.get(0),
        )?;
        format!("{}>{}", parent_path, name)
    } else {
        name.to_string()
    };
    // Append at the end of the chosen parent's child list — get the
    // current max sort_order and add 1. NULL parent_id is matched with
    // IS NULL since `?1 = NULL` is always NULL in SQL.
    let next_order: i64 = match parent_id {
        Some(pid) => conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tags WHERE parent_id = ?1",
            params![pid],
            |r| r.get(0),
        )?,
        None => conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tags WHERE parent_id IS NULL",
            [],
            |r| r.get(0),
        )?,
    };
    conn.execute(
        "INSERT INTO tags (name, parent_id, full_path, sort_order, display) VALUES (?1, ?2, ?3, ?4, 1)",
        params![name, parent_id, full_path, next_order],
    )?;
    Ok(Tag {
        id: conn.last_insert_rowid(),
        name: name.to_string(),
        parent_id,
        full_path,
        children: vec![],
        media_count: 0,
        color: None,
        icon: None,
        display: true,
        sort_order: next_order,
    })
}

pub fn update_tag_appearance(
    conn: &Connection,
    tag_id: i64,
    color: Option<&str>,
    icon: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tags SET color = ?1, icon = ?2 WHERE id = ?3",
        params![color, icon, tag_id],
    )?;
    Ok(())
}

/// Toggle the "show on gallery thumbnail chips" flag.
pub fn set_tag_display(conn: &Connection, tag_id: i64, display: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tags SET display = ?1 WHERE id = ?2",
        params![display as i64, tag_id],
    )?;
    Ok(())
}

pub fn get_tag_tree(conn: &Connection) -> rusqlite::Result<Vec<Tag>> {
    // ORDER BY groups children together (so build_tag_tree picks up
    // each sibling list in the right order) and within each group sorts
    // by the user-controlled `sort_order`. Falls back to name for
    // stability when sort_order values tie (e.g. pre-migration rows).
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.parent_id, t.full_path, COALESCE(c.cnt, 0), \
                t.color, t.icon, COALESCE(t.display, 1), COALESCE(t.sort_order, 0) \
         FROM tags t \
         LEFT JOIN (SELECT tag_id, COUNT(*) as cnt FROM media_tags GROUP BY tag_id) c \
           ON t.id = c.tag_id \
         ORDER BY COALESCE(t.parent_id, 0), COALESCE(t.sort_order, 0), t.name",
    )?;
    let all_tags: Vec<Tag> = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                full_path: row.get(3)?,
                children: vec![],
                media_count: row.get(4)?,
                color: row.get(5)?,
                icon: row.get(6)?,
                display: row.get::<_, i64>(7)? != 0,
                sort_order: row.get(8)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    build_tag_tree(&all_tags)
}

/// Walk `tag_id`'s parent chain (excluding `tag_id` itself) and return the
/// list of ancestor IDs, nearest-parent first.
fn ancestors_of(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut out = Vec::new();
    let mut current: Option<i64> = conn.query_row(
        "SELECT parent_id FROM tags WHERE id = ?1",
        params![tag_id],
        |r| r.get(0),
    )?;
    while let Some(p) = current {
        out.push(p);
        current = conn.query_row(
            "SELECT parent_id FROM tags WHERE id = ?1",
            params![p],
            |r| r.get(0),
        )?;
    }
    Ok(out)
}

/// Re-parent `source` and renumber siblings so its position matches `drop`.
/// Refuses no-op self moves and cycle-creating drops (source becoming a
/// descendant of itself). Recomputes `full_path` for the moved subtree.
///
/// After the move, scrubs auto-applied ancestor rows that are no longer
/// justified: for each ancestor A that source had BEFORE the move but
/// not AFTER, walk every media that still has A assigned with
/// `auto_applied=1` and remove the row when no other tag on that media
/// is a descendant of A. Manual assignments (`auto_applied=0`) are
/// always preserved.
///
/// Returns the IDs of media items whose tag set was modified by the
/// cleanup — caller uses this to drive XMP rewrites.
pub fn move_tag(conn: &Connection, source: i64, drop: TagDropPos) -> rusqlite::Result<Vec<i64>> {
    // Resolve target → (new_parent, neighbour) where `neighbour` is the
    // tag we want `source` placed adjacent to, or None for "append at
    // end" (Into).
    let (new_parent, neighbour, place_after): (Option<i64>, Option<i64>, bool) = match drop {
        TagDropPos::Into(t) => {
            let pid: Option<i64> = None;
            // For Into: new parent is `t` itself.
            let _ = pid;
            (Some(t), None, false)
        }
        TagDropPos::Before(t) => {
            let pid: Option<i64> = conn.query_row(
                "SELECT parent_id FROM tags WHERE id = ?1",
                params![t],
                |r| r.get(0),
            )?;
            (pid, Some(t), false)
        }
        TagDropPos::After(t) => {
            let pid: Option<i64> = conn.query_row(
                "SELECT parent_id FROM tags WHERE id = ?1",
                params![t],
                |r| r.get(0),
            )?;
            (pid, Some(t), true)
        }
    };

    if Some(source) == new_parent {
        return Ok(Vec::new());
    }
    // Cycle check: source must not be (or contain) the new parent.
    if let Some(np) = new_parent {
        if is_descendant_of(conn, np, source)? || np == source {
            return Ok(Vec::new());
        }
    }
    // Trivial no-op: dropping a tag on itself.
    if Some(source) == neighbour {
        return Ok(Vec::new());
    }

    // Capture the ancestor chain BEFORE the re-parent so we can diff
    // against the post-move chain and figure out which ancestors are
    // no longer justified for media that had `source` (or a descendant
    // of `source`) assigned.
    let old_ancestors: std::collections::HashSet<i64> =
        ancestors_of(conn, source)?.into_iter().collect();

    let tx = conn.unchecked_transaction()?;

    // 1. Re-parent the source row.
    tx.execute(
        "UPDATE tags SET parent_id = ?1 WHERE id = ?2",
        params![new_parent, source],
    )?;

    // 2. Build the ordered sibling list under the new parent, excluding source.
    let mut siblings: Vec<i64> = {
        let mut stmt = match new_parent {
            Some(_) => tx.prepare(
                "SELECT id FROM tags WHERE parent_id = ?1 AND id != ?2 \
                 ORDER BY COALESCE(sort_order, 0), name",
            )?,
            None => tx.prepare(
                "SELECT id FROM tags WHERE parent_id IS NULL AND id != ?2 \
                 ORDER BY COALESCE(sort_order, 0), name",
            )?,
        };
        let rows = match new_parent {
            Some(p) => stmt
                .query_map(params![p, source], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
            None => stmt
                .query_map(params![0i64, source], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        };
        rows
    };

    // 3. Decide source's insertion index in the sibling list.
    let insert_at: usize = match neighbour {
        Some(n) => {
            let pos = siblings
                .iter()
                .position(|&id| id == n)
                .unwrap_or(siblings.len());
            if place_after { pos + 1 } else { pos }
        }
        None => siblings.len(), // Into → append.
    };
    let insert_at = insert_at.min(siblings.len());
    siblings.insert(insert_at, source);

    // 4. Renumber every sibling (cheap — bounded by direct children of one parent).
    for (i, sid) in siblings.iter().enumerate() {
        tx.execute(
            "UPDATE tags SET sort_order = ?1 WHERE id = ?2",
            params![i as i64, sid],
        )?;
    }

    // 5. Recompute full_path for the moved subtree (parent change ripples down).
    recompute_subtree_paths(&tx, source)?;

    // 6. Scrub auto-applied ancestors that the move just un-justified.
    let new_ancestors: std::collections::HashSet<i64> =
        ancestors_of(&tx, source)?.into_iter().collect();
    let lost: Vec<i64> = old_ancestors
        .into_iter()
        .filter(|a| !new_ancestors.contains(a))
        .collect();

    let mut affected: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for &a in &lost {
        // Resolve the ancestor's current full_path so we can detect its
        // descendants via `LIKE 'path>%'`. (Paths were just recomputed,
        // so descendants of `source` no longer start with this prefix —
        // exactly what we want.)
        let a_path: String = tx.query_row(
            "SELECT full_path FROM tags WHERE id = ?1",
            params![a],
            |r| r.get(0),
        )?;
        let prefix = format!("{}>%", a_path);

        // Media to clean: those with `a` auto-applied AND no remaining
        // assigned tag whose full_path is a strict descendant of `a`.
        let to_remove: Vec<i64> = {
            let mut stmt = tx.prepare(
                "SELECT mt.media_id FROM media_tags mt \
                 WHERE mt.tag_id = ?1 \
                   AND COALESCE(mt.auto_applied, 0) = 1 \
                   AND NOT EXISTS ( \
                     SELECT 1 FROM media_tags mt2 \
                     JOIN tags t2 ON mt2.tag_id = t2.id \
                     WHERE mt2.media_id = mt.media_id \
                       AND mt2.tag_id != ?1 \
                       AND t2.full_path LIKE ?2 \
                   )",
            )?;
            let v: Vec<i64> = stmt
                .query_map(params![a, prefix], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            v
        };

        if !to_remove.is_empty() {
            let mut del =
                tx.prepare("DELETE FROM media_tags WHERE media_id = ?1 AND tag_id = ?2")?;
            for mid in &to_remove {
                del.execute(params![mid, a])?;
                affected.insert(*mid);
            }
        }
    }

    tx.commit()?;
    Ok(affected.into_iter().collect())
}

/// True if `maybe_descendant` is found anywhere in the subtree rooted at `root`.
fn is_descendant_of(conn: &Connection, maybe_descendant: i64, root: i64) -> rusqlite::Result<bool> {
    let mut current = Some(maybe_descendant);
    while let Some(cid) = current {
        if cid == root {
            return Ok(true);
        }
        current = conn
            .query_row(
                "SELECT parent_id FROM tags WHERE id = ?1",
                params![cid],
                |r| r.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten();
    }
    Ok(false)
}

fn build_tag_tree(tags: &[Tag]) -> rusqlite::Result<Vec<Tag>> {
    let mut roots = Vec::new();
    let mut children_map: std::collections::HashMap<i64, Vec<Tag>> =
        std::collections::HashMap::new();
    for tag in tags.iter().rev() {
        if let Some(pid) = tag.parent_id {
            children_map.entry(pid).or_default().push(tag.clone());
        }
    }
    fn attach_children(
        tag: &Tag,
        children_map: &mut std::collections::HashMap<i64, Vec<Tag>>,
    ) -> Tag {
        let mut t = tag.clone();
        if let Some(children) = children_map.remove(&tag.id) {
            t.children = children
                .iter()
                .rev()
                .map(|c| attach_children(c, children_map))
                .collect();
        }
        t
    }
    for tag in tags {
        if tag.parent_id.is_none() {
            roots.push(attach_children(tag, &mut children_map));
        }
    }
    Ok(roots)
}

/// Update `media.rating` for every id in `media_ids` to `rating`.
/// Clamps to 0..=5; values outside that range are silently capped.
pub fn set_rating_bulk(conn: &Connection, media_ids: &[i64], rating: i32) -> rusqlite::Result<()> {
    if media_ids.is_empty() {
        return Ok(());
    }
    let r = rating.clamp(0, 5);
    let tx = conn.unchecked_transaction()?;
    for &id in media_ids {
        tx.execute("UPDATE media SET rating = ?1 WHERE id = ?2", params![r, id])?;
    }
    tx.commit()
}

pub fn delete_tag(conn: &Connection, tag_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM media_tags WHERE tag_id = ?1", params![tag_id])?;
    conn.execute("DELETE FROM tags WHERE id = ?1", params![tag_id])?;
    Ok(())
}

/// Rename a tag and recompute `full_path` for the tag and every
/// descendant. The full_path schema is `parent>child>grandchild>…`
/// joined by `>`; renaming a non-leaf changes every descendant's
/// canonical path.
pub fn rename_tag(conn: &Connection, tag_id: i64, new_name: &str) -> rusqlite::Result<()> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE tags SET name = ?1 WHERE id = ?2",
        params![new_name, tag_id],
    )?;
    recompute_subtree_paths(&tx, tag_id)?;
    tx.commit()?;
    Ok(())
}

/// Walk a subtree rooted at `tag_id` and rebuild every node's
/// `full_path` from its parent's path + its own name. Used after a
/// rename or a re-parent.
fn recompute_subtree_paths(conn: &Connection, tag_id: i64) -> rusqlite::Result<()> {
    fn recurse(conn: &Connection, tag_id: i64) -> rusqlite::Result<()> {
        let (name, parent_id): (String, Option<i64>) = conn.query_row(
            "SELECT name, parent_id FROM tags WHERE id = ?1",
            params![tag_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let new_path = match parent_id {
            Some(pid) => {
                let pp: String = conn.query_row(
                    "SELECT full_path FROM tags WHERE id = ?1",
                    params![pid],
                    |r| r.get(0),
                )?;
                format!("{}>{}", pp, name)
            }
            None => name.clone(),
        };
        conn.execute(
            "UPDATE tags SET full_path = ?1 WHERE id = ?2",
            params![new_path, tag_id],
        )?;
        let mut stmt = conn.prepare("SELECT id FROM tags WHERE parent_id = ?1")?;
        let child_ids: Vec<i64> = stmt
            .query_map(params![tag_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for cid in child_ids {
            recurse(conn, cid)?;
        }
        Ok(())
    }
    recurse(conn, tag_id)
}

/// Merge `source_id` into `target_id`. Every media item tagged with the
/// source is also given the target tag (skipping duplicates), then the
/// source's media-tag links are dropped. `reparent_children` controls
/// the source's children: when true they're re-parented under the
/// target (preserving their data); when false they're deleted via the
/// `ON DELETE CASCADE` triggered by removing the source.
///
/// Returns Err if the operation would create a duplicate `full_path`
/// (e.g. both source and target have a child of the same name and we're
/// re-parenting). Caller can surface that to the user.
pub fn merge_tag(
    conn: &Connection,
    source_id: i64,
    target_id: i64,
    reparent_children: bool,
) -> rusqlite::Result<()> {
    if source_id == target_id {
        return Ok(());
    }
    let tx = conn.unchecked_transaction()?;
    // 1. Hand the source's media over to the target. INSERT OR IGNORE
    //    so items already tagged with both don't violate the PK.
    tx.execute(
        "INSERT OR IGNORE INTO media_tags (media_id, tag_id)
         SELECT media_id, ?1 FROM media_tags WHERE tag_id = ?2",
        params![target_id, source_id],
    )?;
    tx.execute(
        "DELETE FROM media_tags WHERE tag_id = ?1",
        params![source_id],
    )?;

    if reparent_children {
        // 2a. Re-parent direct children of source under target.
        tx.execute(
            "UPDATE tags SET parent_id = ?1 WHERE parent_id = ?2",
            params![target_id, source_id],
        )?;
        // 2b. Recompute full_paths for every former-source-child subtree.
        let mut stmt = tx.prepare("SELECT id FROM tags WHERE parent_id = ?1")?;
        let new_children: Vec<i64> = stmt
            .query_map(params![target_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        for cid in new_children {
            recompute_subtree_paths(&tx, cid)?;
        }
    }
    // (When !reparent_children, `ON DELETE CASCADE` on tags.parent_id
    // will recursively delete the source's descendants in the next step.)

    // 3. Delete the source tag itself.
    tx.execute("DELETE FROM tags WHERE id = ?1", params![source_id])?;
    tx.commit()?;
    Ok(())
}
