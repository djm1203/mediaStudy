use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

use crate::bucket::{self, Bucket};
use crate::config::Config;

/// App-level logical schema version, stored in `meta.schema_version` (B-021).
/// Bump when a change needs a data migration; add the migration step alongside.
pub const SCHEMA_VERSION: i64 = 1;

pub struct Database {
    pub conn: Connection,
    #[allow(dead_code)]
    pub path: PathBuf,
}

impl Database {
    /// Open or create the database for the current bucket (or default if no bucket)
    pub fn open() -> Result<Self> {
        let path = match bucket::get_current_bucket()? {
            Some(bucket) => bucket.db_path(),
            None => Self::default_db_path()?,
        };

        Self::open_at_path(path)
    }

    /// Open or create a database for a specific bucket
    pub fn open_for_bucket(bucket: &Bucket) -> Result<Self> {
        Self::open_at_path(bucket.db_path())
    }

    /// Open or create a database at a specific path
    pub fn open_at_path(path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create data directory: {:?}", parent))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open database: {:?}", path))?;

        let db = Self { conn, path };
        db.init_schema()?;

        Ok(db)
    }

    /// Get the default database file path (when no bucket is selected)
    fn default_db_path() -> Result<PathBuf> {
        Ok(Config::data_dir()?.join("default.db"))
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        // Documents table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_path TEXT NOT NULL,
                filename TEXT NOT NULL,
                content_type TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Full-text search virtual table
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                filename,
                content,
                tags,
                content='documents',
                content_rowid='id'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
                INSERT INTO documents_fts(rowid, filename, content, tags)
                VALUES (new.id, new.filename, new.content, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags)
                VALUES ('delete', old.id, old.filename, old.content, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
                INSERT INTO documents_fts(documents_fts, rowid, filename, content, tags)
                VALUES ('delete', old.id, old.filename, old.content, old.tags);
                INSERT INTO documents_fts(rowid, filename, content, tags)
                VALUES (new.id, new.filename, new.content, new.tags);
            END;
            ",
        )?;

        // Conversations table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Messages table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Additive migration: content_hash on documents for content-based dedup
        // (B-019). Older databases predate the column, so add it if missing.
        let has_content_hash: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('documents') WHERE name = 'content_hash'",
            [],
            |row| row.get(0),
        )?;
        if has_content_hash == 0 {
            self.conn
                .execute("ALTER TABLE documents ADD COLUMN content_hash TEXT", [])?;
        }

        // Study items table (spaced repetition)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS study_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id INTEGER,
                item_type TEXT NOT NULL,
                front TEXT NOT NULL,
                back TEXT NOT NULL,
                next_review_date TEXT NOT NULL,
                interval_days REAL NOT NULL DEFAULT 1.0,
                ease_factor REAL NOT NULL DEFAULT 2.5,
                review_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE SET NULL
            )",
            [],
        )?;

        // Key/value metadata (B-021): app-level schema version, the embedding
        // model identity used to build the vectors, etc. This is the extensible
        // home for schema/migration state. (Note: the `chunks_fts` backfill uses
        // `PRAGMA user_version` as its own internal one-shot gate — see
        // `ChunkStore::init_fts` — kept separate so a bump here can never skip
        // that backfill on a legacy database.)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        self.meta_set_if_absent("schema_version", &SCHEMA_VERSION.to_string())?;

        Ok(())
    }

    /// Read a value from the `meta` key/value table.
    pub fn meta_get(&self, key: &str) -> Result<Option<String>> {
        let value = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get::<_, String>(0)
            })
            .ok();
        Ok(value)
    }

    /// Upsert a value into the `meta` key/value table.
    pub fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    /// Set a meta value only if the key is not already present.
    fn meta_set_if_absent(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key, value) VALUES(?1, ?2)",
            [key, value],
        )?;
        Ok(())
    }
}
