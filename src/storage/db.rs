use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

use crate::bucket::{self, Bucket};
use crate::config::Config;

pub struct Database {
    pub conn: Connection,
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
            "
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

        Ok(())
    }
}
