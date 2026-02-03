use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Database;

#[derive(Debug, Clone)]
pub struct Document {
    pub id: i64,
    pub source_path: String,
    pub filename: String,
    pub content_type: String,
    pub content: String,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct DocumentStore<'a> {
    db: &'a Database,
}

impl<'a> DocumentStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Insert a new document
    pub fn insert(
        &self,
        source_path: &str,
        filename: &str,
        content_type: &str,
        content: &str,
        tags: Option<&str>,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();

        self.db.conn.execute(
            "INSERT INTO documents (source_path, filename, content_type, content, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![source_path, filename, content_type, content, tags, now, now],
        ).context("Failed to insert document")?;

        Ok(self.db.conn.last_insert_rowid())
    }

    /// Get a document by ID
    pub fn get(&self, id: i64) -> Result<Option<Document>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, source_path, filename, content_type, content, tags, created_at, updated_at
             FROM documents WHERE id = ?1"
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
             FROM documents ORDER BY created_at DESC"
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
        let affected = self.db.conn.execute(
            "DELETE FROM documents WHERE id = ?1",
            params![id],
        )?;

        Ok(affected > 0)
    }

    /// Get document count
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0),
        )?;

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
