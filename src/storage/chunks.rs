use anyhow::{Context, Result};
use rusqlite::params;

use super::Database;
use crate::embeddings;

/// A stored chunk with its embedding
#[derive(Debug, Clone)]
pub struct StoredChunk {
    pub id: i64,
    pub document_id: i64,
    pub chunk_index: i64,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
}

pub struct ChunkStore<'a> {
    db: &'a Database,
}

impl<'a> ChunkStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Initialize chunks table if not exists
    pub fn init_schema(&self) -> Result<()> {
        self.db.conn.execute(
            "CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id INTEGER NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )",
            [],
        )?;

        self.db.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_document_id ON chunks(document_id)",
            [],
        )?;

        Ok(())
    }

    /// Insert a chunk
    pub fn insert(
        &self,
        document_id: i64,
        chunk_index: i64,
        content: &str,
        embedding: Option<&[f32]>,
    ) -> Result<i64> {
        let embedding_bytes = embedding.map(embeddings::embedding_to_bytes);

        self.db
            .conn
            .execute(
                "INSERT INTO chunks (document_id, chunk_index, content, embedding)
             VALUES (?1, ?2, ?3, ?4)",
                params![document_id, chunk_index, content, embedding_bytes],
            )
            .context("Failed to insert chunk")?;

        Ok(self.db.conn.last_insert_rowid())
    }

    /// Get all chunks for a document
    #[allow(dead_code)]
    pub fn get_for_document(&self, document_id: i64) -> Result<Vec<StoredChunk>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, document_id, chunk_index, content, embedding
             FROM chunks WHERE document_id = ?1 ORDER BY chunk_index",
        )?;

        let rows = stmt.query_map(params![document_id], |row| {
            let embedding_bytes: Option<Vec<u8>> = row.get(4)?;
            let embedding = embedding_bytes.map(|b| embeddings::bytes_to_embedding(&b));

            Ok(StoredChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                embedding,
            })
        })?;

        let mut chunks = Vec::new();
        for chunk in rows {
            chunks.push(chunk?);
        }

        Ok(chunks)
    }

    /// Get all chunks with embeddings (for semantic search)
    pub fn get_all_with_embeddings(&self) -> Result<Vec<StoredChunk>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, document_id, chunk_index, content, embedding
             FROM chunks WHERE embedding IS NOT NULL",
        )?;

        let rows = stmt.query_map([], |row| {
            let embedding_bytes: Option<Vec<u8>> = row.get(4)?;
            let embedding = embedding_bytes.map(|b| embeddings::bytes_to_embedding(&b));

            Ok(StoredChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                embedding,
            })
        })?;

        let mut chunks = Vec::new();
        for chunk in rows {
            chunks.push(chunk?);
        }

        Ok(chunks)
    }

    /// Delete chunks for a document
    #[allow(dead_code)]
    pub fn delete_for_document(&self, document_id: i64) -> Result<usize> {
        let affected = self.db.conn.execute(
            "DELETE FROM chunks WHERE document_id = ?1",
            params![document_id],
        )?;

        Ok(affected)
    }

    /// Count chunks for a document
    #[allow(dead_code)]
    pub fn count_for_document(&self, document_id: i64) -> Result<i64> {
        let count: i64 = self.db.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE document_id = ?1",
            params![document_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Count total chunks
    pub fn count(&self) -> Result<i64> {
        let count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;

        Ok(count)
    }

    /// Get chunks with embeddings that haven't been embedded yet
    #[allow(dead_code)]
    pub fn get_unembedded(&self) -> Result<Vec<StoredChunk>> {
        let mut stmt = self.db.conn.prepare(
            "SELECT id, document_id, chunk_index, content, embedding
             FROM chunks WHERE embedding IS NULL",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StoredChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                embedding: None,
            })
        })?;

        let mut chunks = Vec::new();
        for chunk in rows {
            chunks.push(chunk?);
        }

        Ok(chunks)
    }

    /// Update chunk embedding
    #[allow(dead_code)]
    pub fn update_embedding(&self, chunk_id: i64, embedding: &[f32]) -> Result<()> {
        let embedding_bytes = embeddings::embedding_to_bytes(embedding);

        self.db.conn.execute(
            "UPDATE chunks SET embedding = ?1 WHERE id = ?2",
            params![embedding_bytes, chunk_id],
        )?;

        Ok(())
    }
}
