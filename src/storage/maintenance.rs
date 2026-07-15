//! Bucket maintenance: stats, compaction, and portable export (B-020).
//!
//! Export uses SQLite's `VACUUM INTO`, which writes a fresh, fully-compacted
//! copy of the database to a new file — so a single primitive gives us both the
//! portable-archive export *and* compaction. A `.db` file is self-contained and
//! portable, so no archive format/dependency is needed.

use std::path::Path;

use anyhow::{Context, Result, bail};
use rusqlite::params;

use super::Database;

/// A snapshot of a bucket's contents for `librarian stats`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub documents: i64,
    pub chunks: i64,
    /// Chunks that have an embedding vector (the rest are keyword-only).
    pub embedded_chunks: i64,
    pub study_items: i64,
    pub conversations: i64,
    /// On-disk size of the database file, in bytes.
    pub db_bytes: u64,
}

/// Gather counts + on-disk size for the given database.
pub fn gather_stats(db: &Database) -> Result<Stats> {
    let documents = count(db, "SELECT COUNT(*) FROM documents")?;
    // The chunks table is created lazily, so tolerate its absence.
    let chunks = count_opt(db, "SELECT COUNT(*) FROM chunks")?;
    let embedded_chunks = count_opt(
        db,
        "SELECT COUNT(*) FROM chunks WHERE embedding IS NOT NULL",
    )?;
    let study_items = count(db, "SELECT COUNT(*) FROM study_items")?;
    let conversations = count(db, "SELECT COUNT(*) FROM conversations")?;
    let db_bytes = std::fs::metadata(&db.path).map(|m| m.len()).unwrap_or(0);

    Ok(Stats {
        documents,
        chunks,
        embedded_chunks,
        study_items,
        conversations,
        db_bytes,
    })
}

/// Compact the database in place (reclaims free pages).
pub fn vacuum(db: &Database) -> Result<()> {
    db.conn.execute("VACUUM", [])?;
    Ok(())
}

/// Write a compacted, portable copy of the database to `dest` (export).
///
/// `VACUUM INTO` requires that `dest` does not already exist.
pub fn vacuum_into(db: &Database, dest: &Path) -> Result<()> {
    if dest.exists() {
        bail!("destination already exists: {}", dest.display());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create export directory: {:?}", parent))?;
    }
    let dest_str = dest.to_string_lossy().to_string();
    db.conn
        .execute("VACUUM INTO ?1", params![dest_str])
        .with_context(|| format!("Failed to export database to {}", dest.display()))?;
    Ok(())
}

fn count(db: &Database, sql: &str) -> Result<i64> {
    Ok(db.conn.query_row(sql, [], |row| row.get(0))?)
}

/// Like [`count`] but yields 0 if the table does not exist yet.
fn count_opt(db: &Database, sql: &str) -> Result<i64> {
    match db.conn.query_row(sql, [], |row| row.get(0)) {
        Ok(n) => Ok(n),
        Err(_) => Ok(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{ChunkStore, DocumentStore};

    fn temp_db(name: &str) -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_at_path(dir.path().join(name)).unwrap();
        (dir, db)
    }

    #[test]
    fn stats_counts_documents_and_chunks() {
        let (_dir, db) = temp_db("stats.db");
        let doc = DocumentStore::new(&db)
            .insert("/p/a.md", "a.md", "text", "hello world", None)
            .unwrap();
        let chunks = ChunkStore::new(&db);
        chunks.init_schema().unwrap();
        chunks.insert(doc, 0, "hello", Some(&[0.1, 0.2])).unwrap();
        chunks.insert(doc, 1, "world", None).unwrap();

        let s = gather_stats(&db).unwrap();
        assert_eq!(s.documents, 1);
        assert_eq!(s.chunks, 2);
        assert_eq!(s.embedded_chunks, 1);
        assert!(s.db_bytes > 0);
    }

    #[test]
    fn meta_roundtrips_and_seeds_schema_version() {
        let (_dir, db) = temp_db("meta.db");
        // schema_version is seeded on init and not overwritten on reopen.
        assert_eq!(
            db.meta_get("schema_version").unwrap().as_deref(),
            Some(super::super::db::SCHEMA_VERSION.to_string().as_str())
        );
        assert_eq!(db.meta_get("embedding_model").unwrap(), None);
        db.meta_set("embedding_model", "all-MiniLM-L6-v2").unwrap();
        assert_eq!(
            db.meta_get("embedding_model").unwrap().as_deref(),
            Some("all-MiniLM-L6-v2")
        );
        // Upsert overwrites.
        db.meta_set("embedding_model", "other").unwrap();
        assert_eq!(
            db.meta_get("embedding_model").unwrap().as_deref(),
            Some("other")
        );
    }

    #[test]
    fn export_produces_a_usable_copy() {
        let (dir, db) = temp_db("src.db");
        DocumentStore::new(&db)
            .insert("/p/a.md", "a.md", "text", "exported content", None)
            .unwrap();

        let dest = dir.path().join("out/export.db");
        vacuum_into(&db, &dest).unwrap();
        assert!(dest.exists());

        // The exported copy opens and carries the same data.
        let copy = Database::open_at_path(dest.clone()).unwrap();
        assert_eq!(gather_stats(&copy).unwrap().documents, 1);

        // A second export to the same path is refused rather than clobbering.
        assert!(vacuum_into(&db, &dest).is_err());
    }
}
