#[cfg(feature = "test-api")]
pub mod queries;
#[cfg(not(feature = "test-api"))]
pub(crate) mod queries;

use rusqlite::Connection;

fn db_path() -> std::path::PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dir = base.join("PhoTag");
    std::fs::create_dir_all(&dir).ok();
    dir.join("photag.db")
}

pub fn open_db() -> rusqlite::Result<Connection> {
    open_db_at(db_path())
}

/// Open and initialize a PhoTag database at an explicit path.
pub fn open_db_at(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    initialize(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open an additional connection to the same DB. Used so the UI thread can
/// read while the scan thread holds the write connection — SQLite's WAL
/// journal allows concurrent readers + one writer at the file level, but
/// only when they're separate Connection objects.
pub fn open_read_conn() -> rusqlite::Result<Connection> {
    open_read_conn_at(db_path())
}

/// Open a separately tuned read connection to an explicit PhoTag database.
pub fn open_read_conn_at(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // WAL is set on the writer; both connections see it. Tune for fast reads.
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
        .ok();
    Ok(conn)
}

pub(crate) fn initialize(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        -- NORMAL is safe with WAL: durable on commit, only loses last txn on
        -- power loss (not on app crash). Avoids fsync per commit, which
        -- otherwise stalls the scan flusher every 16-row batch.
        PRAGMA synchronous = NORMAL;
        -- Checkpoint when WAL exceeds ~4 MB (1000 pages × 4 KB). Keeps the
        -- WAL file small so reader queries stay fast as the scan progresses.
        PRAGMA wal_autocheckpoint = 1000;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS folders (
            id          INTEGER PRIMARY KEY,
            path        TEXT NOT NULL UNIQUE,
            last_scan   INTEGER
        );

        CREATE TABLE IF NOT EXISTS media (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            folder_id       INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            filename        TEXT NOT NULL,
            full_path       TEXT NOT NULL UNIQUE,
            media_type      TEXT NOT NULL,
            file_size       INTEGER,
            modified_at     INTEGER,
            width           INTEGER,
            height          INTEGER,
            date_taken      INTEGER,
            thumb_path      TEXT,
            metadata_dirty  INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS tags (
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            parent_id   INTEGER REFERENCES tags(id) ON DELETE CASCADE,
            full_path   TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS media_tags (
            media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
            tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (media_id, tag_id)
        );

        CREATE INDEX IF NOT EXISTS idx_media_folder ON media(folder_id);
        CREATE INDEX IF NOT EXISTS idx_media_modified ON media(modified_at);
        CREATE INDEX IF NOT EXISTS idx_media_tags_tag ON media_tags(tag_id);
        CREATE INDEX IF NOT EXISTS idx_media_tags_media ON media_tags(media_id);
        CREATE INDEX IF NOT EXISTS idx_tags_parent ON tags(parent_id);

        CREATE TABLE IF NOT EXISTS versions (
            entity_id TEXT NOT NULL,
            version_id TEXT NOT NULL,
            revision_number INTEGER NOT NULL,
            parents TEXT NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            content_type TEXT,
            thumbnail_path TEXT,
            is_current INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (entity_id, version_id)
        );
        CREATE INDEX IF NOT EXISTS idx_versions_entity_revision
            ON versions(entity_id, revision_number DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_versions_current
            ON versions(entity_id) WHERE is_current=1;
        CREATE TABLE IF NOT EXISTS version_graph (
            entity_id TEXT PRIMARY KEY,
            snapshot TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS version_metadata_imports (
            legacy_path TEXT PRIMARY KEY,
            imported_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS app_launcher_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            position INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS app_launcher_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            discovery_key TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            launch_path TEXT NOT NULL,
            uninstall_command TEXT,
            category_id INTEGER NOT NULL,
            hidden INTEGER NOT NULL DEFAULT 0,
            custom_name INTEGER NOT NULL DEFAULT 0,
            custom_category INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'manual'
        );
        CREATE TABLE IF NOT EXISTS app_launcher_metadata (
            entry_id INTEGER PRIMARY KEY REFERENCES app_launcher_entries(id) ON DELETE CASCADE,
            installed_on TEXT,
            size_bytes INTEGER,
            pin_scope INTEGER NOT NULL DEFAULT 0 CHECK(pin_scope BETWEEN 0 AND 2)
        );
        CREATE TABLE IF NOT EXISTS clipboard_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            position INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL UNIQUE,
            category_id INTEGER REFERENCES clipboard_categories(id) ON DELETE SET NULL,
            pinned INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_used_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_clipboard_history_recent ON clipboard_history(pinned DESC, last_used_at DESC);
    ",
    )
}

/// Run schema migrations for new columns added after initial release.
pub(crate) fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    for (column, declaration) in [
        ("content_type", "TEXT NOT NULL DEFAULT 'text'"),
        ("image_path", "TEXT"),
        ("image_width", "INTEGER"),
        ("image_height", "INTEGER"),
    ] {
        if conn
            .prepare(&format!("SELECT {column} FROM clipboard_history LIMIT 0"))
            .is_err()
        {
            conn.execute_batch(&format!(
                "ALTER TABLE clipboard_history ADD COLUMN {column} {declaration}"
            ))?;
        }
    }

    let has_launcher_pin_scope = conn
        .prepare("SELECT pin_scope FROM app_launcher_metadata LIMIT 0")
        .is_ok();
    if !has_launcher_pin_scope {
        conn.execute_batch(
            "ALTER TABLE app_launcher_metadata ADD COLUMN pin_scope INTEGER NOT NULL DEFAULT 0 CHECK(pin_scope BETWEEN 0 AND 2)",
        )?;
    }

    // Optimization #2: micro_thumb BLOB — tiny 8x8 placeholder thumbnail
    let has_micro = conn
        .prepare("SELECT micro_thumb FROM media LIMIT 0")
        .is_ok();
    if !has_micro {
        conn.execute_batch("ALTER TABLE media ADD COLUMN micro_thumb BLOB")?;
    }

    // Tag appearance: per-tag color (hex "#RRGGBB") and icon (short emoji/char).
    let has_color = conn.prepare("SELECT color FROM tags LIMIT 0").is_ok();
    if !has_color {
        conn.execute_batch("ALTER TABLE tags ADD COLUMN color TEXT")?;
    }
    let has_icon = conn.prepare("SELECT icon FROM tags LIMIT 0").is_ok();
    if !has_icon {
        conn.execute_batch("ALTER TABLE tags ADD COLUMN icon TEXT")?;
    }

    // open_count: number of times the user has opened this media item in the
    // preview (image preview or video player). Surfaced in the info panel.
    let has_open_count = conn.prepare("SELECT open_count FROM media LIMIT 0").is_ok();
    if !has_open_count {
        conn.execute_batch("ALTER TABLE media ADD COLUMN open_count INTEGER DEFAULT 0")?;
    }

    // Rating (0..=5) and pick flag (-1 reject, 0 none, 1 pick) — drives
    // the photo-culling workflow (1-5 / P / X hotkeys, rating sort + filter).
    let has_rating = conn.prepare("SELECT rating FROM media LIMIT 0").is_ok();
    if !has_rating {
        conn.execute_batch("ALTER TABLE media ADD COLUMN rating INTEGER DEFAULT 0")?;
    }
    let has_pick = conn.prepare("SELECT pick_flag FROM media LIMIT 0").is_ok();
    if !has_pick {
        conn.execute_batch("ALTER TABLE media ADD COLUMN pick_flag INTEGER DEFAULT 0")?;
    }

    // Per-tag "display on thumbnail chips" toggle. Defaults to 1 so every
    // existing tag stays visible after upgrade.
    let has_display = conn.prepare("SELECT display FROM tags LIMIT 0").is_ok();
    if !has_display {
        conn.execute_batch("ALTER TABLE tags ADD COLUMN display INTEGER DEFAULT 1")?;
    }
    // Per-tag sibling ordering, set by drag-reorder in the sidebar. Pre-
    // migration rows all get 0; we seed them from name order so the
    // existing alphabetic look is preserved until the user reorders.
    // Tracks whether a (media, tag) assignment was made directly by the user
    // (=0, manual) or implicitly via ancestor expansion when assigning a
    // descendant (=1, auto). Pre-migration rows all get 0 so existing
    // assignments are treated as manual — they will never be auto-removed.
    let has_auto = conn
        .prepare("SELECT auto_applied FROM media_tags LIMIT 0")
        .is_ok();
    if !has_auto {
        conn.execute_batch("ALTER TABLE media_tags ADD COLUMN auto_applied INTEGER DEFAULT 0")?;
    }

    let has_sort_order = conn.prepare("SELECT sort_order FROM tags LIMIT 0").is_ok();
    if !has_sort_order {
        conn.execute_batch("ALTER TABLE tags ADD COLUMN sort_order INTEGER DEFAULT 0")?;
        // Seed sort_order per parent group by name. Using a correlated
        // subquery so SQLite can do it in one shot.
        conn.execute_batch(
            "UPDATE tags SET sort_order = (
                SELECT COUNT(*) FROM tags t2
                 WHERE (t2.parent_id IS tags.parent_id)
                   AND (t2.name < tags.name OR (t2.name = tags.name AND t2.id < tags.id))
             )",
        )?;
    }

    // Associated tags: when tag A is assigned, also auto-assign its associates.
    let has_assoc = conn
        .prepare("SELECT 1 FROM tag_associations LIMIT 0")
        .is_ok();
    if !has_assoc {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tag_associations (
                tag_id            INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                associated_tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                PRIMARY KEY (tag_id, associated_tag_id)
            )",
        )?;
    }

    Ok(())
}

/// Read the open_count for a single media row. Returns 0 on any error
/// (missing column on a freshly-migrated DB, missing row, etc.).
pub fn get_open_count(conn: &Connection, media_id: i64) -> i64 {
    conn.query_row(
        "SELECT COALESCE(open_count, 0) FROM media WHERE id = ?1",
        rusqlite::params![media_id],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
}
