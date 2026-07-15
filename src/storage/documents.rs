use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;
use sha2::{Digest, Sha256};

use super::Database;

/// SHA-256 hex digest of a document's text, used for content-based dedup (B-019).
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    #[allow(dead_code)]
    pub source_path: String,
    pub filename: String,
    pub content_type: String,
    pub content: String,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

pub struct DocumentStore<'a> {
    db: &'a Database,
}

impl<'a> DocumentStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert a new document. The content hash is computed and stored so
    /// identical content can be de-duplicated later (B-019).
    pub fn insert(
        &self,
        source_path: &str,
        filename: &str,
        content_type: &str,
        content: &str,
        tags: Option<&str>,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let hash = hash_content(content);

        self.db.conn.execute(
            "INSERT INTO documents (source_path, filename, content_type, content, tags, created_at, updated_at, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![source_path, filename, content_type, content, tags, now, now, hash],
        ).context("Failed to insert document")?;

        Ok(self.db.conn.last_insert_rowid())
    }

    /// Get a document by ID
    pub fn get(&self, id: i64) -> Result<Option<Document>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, source_path, filename, content_type, content, tags, created_at, updated_at
             FROM documents WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_document(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all documents
    pub fn list(&self) -> Result<Vec<Document>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, source_path, filename, content_type, content, tags, created_at, updated_at
             FROM documents ORDER BY created_at DESC",
        )?;

        let mut rows = stmt.query([])?;
        let mut documents = Vec::new();

        while let Some(row) = rows.next()? {
            documents.push(Self::row_to_document(row)?);
        }

        Ok(documents)
    }

    /// Search documents using full-text search
    pub fn search(&self, query: &str) -> Result<Vec<Document>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT d.id, d.source_path, d.filename, d.content_type, d.content, d.tags, d.created_at, d.updated_at
             FROM documents d
             JOIN documents_fts fts ON d.id = fts.rowid
             WHERE documents_fts MATCH ?1
             ORDER BY rank"
        )?;

        let mut rows = stmt.query(params![query])?;
        let mut documents = Vec::new();

        while let Some(row) = rows.next()? {
            documents.push(Self::row_to_document(row)?);
        }

        Ok(documents)
    }

    /// Delete a document by ID
    pub fn delete(&self, id: i64) -> Result<bool> {
        let affected = self
            .db
            .conn
            .execute("DELETE FROM documents WHERE id = ?1", params![id])?;

        Ok(affected > 0)
    }

    /// Get document count
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;

        Ok(count)
    }

    /// Check if a document with the given source path already exists
    pub fn exists_by_path(&self, source_path: &str) -> Result<bool> {
        let count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE source_path = ?1",
            params![source_path],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Whether a document with byte-identical content already exists (B-019),
    /// regardless of its source path — so the same file re-added under a new
    /// name/location, or an identical copy, is not re-ingested/re-embedded.
    pub fn exists_by_content(&self, content: &str) -> Result<bool> {
        let hash = hash_content(content);
        let count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE content_hash = ?1",
            params![hash],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    fn row_to_document(row: &rusqlite::Row) -> Result<Document> {
        let created_str: String = row.get(6)?;
        let updated_str: String = row.get(7)?;

        Ok(Document {
            id: row.get(0)?,
            source_path: row.get(1)?,
            filename: row.get(2)?,
            content_type: row.get(3)?,
            content: row.get(4)?,
            tags: row.get(5)?,
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .context("Invalid created_at timestamp")?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .context("Invalid updated_at timestamp")?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DocumentStore;
    use crate::storage::Database;

    fn temp_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_at_path(dir.path().join("docs.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn document_crud_roundtrip() {
        let (_dir, db) = temp_db();
        let store = DocumentStore::new(&db);

        let id = store
            .insert("/path/a.md", "a.md", "text", "alpha content", Some("t1"))
            .unwrap();
        assert_eq!(store.count().unwrap(), 1);
        assert!(store.exists_by_path("/path/a.md").unwrap());
        assert!(!store.exists_by_path("/path/missing.md").unwrap());

        let got = store.get(id).unwrap().unwrap();
        assert_eq!(got.filename, "a.md");
        assert_eq!(got.content, "alpha content");
        assert_eq!(got.tags.as_deref(), Some("t1"));

        assert!(store.delete(id).unwrap());
        assert_eq!(store.count().unwrap(), 0);
        assert!(store.get(id).unwrap().is_none());
    }

    #[test]
    fn fts_search_matches_content_and_syncs_on_delete() {
        let (_dir, db) = temp_db();
        let store = DocumentStore::new(&db);
        let bio = store
            .insert(
                "/p/bio.md",
                "bio.md",
                "text",
                "mitochondria and enzymes",
                None,
            )
            .unwrap();
        store
            .insert(
                "/p/geo.md",
                "geo.md",
                "text",
                "tectonic plates and faults",
                None,
            )
            .unwrap();

        let hits = store.search("enzymes").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].filename, "bio.md");

        // The FTS delete trigger must keep the index consistent.
        store.delete(bio).unwrap();
        assert!(store.search("enzymes").unwrap().is_empty());
    }

    #[test]
    fn dedup_by_content_hash_ignores_path() {
        let (_dir, db) = temp_db();
        let store = DocumentStore::new(&db);
        store
            .insert(
                "/p/original.md",
                "original.md",
                "text",
                "identical body",
                None,
            )
            .unwrap();

        // Same content, different path → detected as a content duplicate…
        assert!(store.exists_by_content("identical body").unwrap());
        // …but the path-based check does not see the new path.
        assert!(!store.exists_by_path("/p/copy.md").unwrap());
        // Different content is not a duplicate.
        assert!(!store.exists_by_content("something else").unwrap());
    }

    #[test]
    fn list_orders_newest_first() {
        let (_dir, db) = temp_db();
        let store = DocumentStore::new(&db);
        store
            .insert("/p/1.md", "1.md", "text", "one", None)
            .unwrap();
        // Guarantee a distinct created_at so the DESC ordering is unambiguous
        // regardless of the platform clock's resolution.
        std::thread::sleep(std::time::Duration::from_millis(5));
        store
            .insert("/p/2.md", "2.md", "text", "two", None)
            .unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
        // created_at DESC — most recently inserted first.
        assert_eq!(list[0].filename, "2.md");
    }
}
