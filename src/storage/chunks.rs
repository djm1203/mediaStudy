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

/// Database schema version once the `chunks_fts` index exists and is backfilled
/// (stored in `PRAGMA user_version`). Bump when adding future migrations.
const CHUNKS_FTS_SCHEMA_VERSION: i64 = 1;

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

        self.init_fts()?;

        Ok(())
    }

    /// Create the `chunks_fts` FTS5 table (external-content, mirroring
    /// `documents_fts`) with sync triggers, and backfill it from any chunks that
    /// predate the index. (B-001 — promotes the keyword arm from `LIKE` to FTS5.)
    fn init_fts(&self) -> Result<()> {
        self.db.conn.execute_batch(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                content,
                content='chunks',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
                INSERT INTO chunks_fts(rowid, content) VALUES (new.id, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
                INSERT INTO chunks_fts(chunks_fts, rowid, content)
                VALUES ('delete', old.id, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
                INSERT INTO chunks_fts(chunks_fts, rowid, content)
                VALUES ('delete', old.id, old.content);
                INSERT INTO chunks_fts(rowid, content) VALUES (new.id, new.content);
            END;
            ",
        )?;

        // Backfill via a schema-version gate. `COUNT(*)` on an external-content
        // FTS5 table reads through to the base table, so it can't tell an empty
        // index from a full one; instead we use `PRAGMA user_version` as a
        // one-shot migration marker (also the seed for B-021 schema versioning).
        // Version 0 = pre-`chunks_fts`, so rebuild the index from existing rows
        // (harmless on a brand-new empty table) and bump to 1.
        let version: i64 = self
            .db
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < CHUNKS_FTS_SCHEMA_VERSION {
            self.db
                .conn
                .execute("INSERT INTO chunks_fts(chunks_fts) VALUES('rebuild')", [])?;
            self.db.conn.execute_batch(&format!(
                "PRAGMA user_version = {CHUNKS_FTS_SCHEMA_VERSION}"
            ))?;
        }

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

    /// Search chunks by keyword (LIKE matching for hybrid search)
    pub fn search_content(&self, query: &str, limit: usize) -> Result<Vec<StoredChunk>> {
        // Split query into keywords and search for any match
        let keywords: Vec<&str> = query.split_whitespace().filter(|w| w.len() >= 2).collect();

        if keywords.is_empty() {
            return Ok(Vec::new());
        }

        // Build a query that matches ANY keyword
        let conditions: Vec<String> = keywords
            .iter()
            .map(|_| "content LIKE ?".to_string())
            .collect();
        let where_clause = conditions.join(" OR ");

        let sql = format!(
            "SELECT id, document_id, chunk_index, content, embedding
             FROM chunks WHERE {} LIMIT ?",
            where_clause
        );

        let mut stmt = self.db.conn.prepare(&sql)?;

        // Bind parameters: each keyword as %keyword%, then limit
        let mut param_idx = 1;
        for kw in &keywords {
            stmt.raw_bind_parameter(param_idx, format!("%{}%", kw))?;
            param_idx += 1;
        }
        stmt.raw_bind_parameter(param_idx, limit as i64)?;

        let mut chunks = Vec::new();
        let mut rows = stmt.raw_query();
        while let Some(row) = rows.next()? {
            let embedding_bytes: Option<Vec<u8>> = row.get(4)?;
            let embedding = embedding_bytes.map(|b| embeddings::bytes_to_embedding(&b));

            chunks.push(StoredChunk {
                id: row.get(0)?,
                document_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                embedding,
            });
        }

        Ok(chunks)
    }

    /// Search chunks by keyword using the `chunks_fts` FTS5 index, ranked by
    /// relevance (best first). Returns chunks in rank order. (B-001)
    ///
    /// The raw query is sanitized into a safe FTS5 `MATCH` expression (each term
    /// quoted as a string literal, OR-joined) so punctuation like `0.3` or `?`
    /// never produces a syntax error. Returns `Ok(vec![])` when the query has no
    /// usable terms; callers fall back to [`ChunkStore::search_content`] on error.
    pub fn search_content_fts(&self, query: &str, limit: usize) -> Result<Vec<StoredChunk>> {
        let Some(match_expr) = fts_match_expr(query) else {
            return Ok(Vec::new());
        };

        let mut stmt = self.db.conn.prepare(
            "SELECT c.id, c.document_id, c.chunk_index, c.content, c.embedding
             FROM chunks c
             JOIN chunks_fts fts ON c.id = fts.rowid
             WHERE chunks_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![match_expr, limit as i64], |row| {
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

    /// Get `(id, content)` for every chunk, for a full re-embed (B-021).
    pub fn get_all_for_reembed(&self) -> Result<Vec<(i64, String)>> {
        let mut stmt = self
            .db
            .conn
            .prepare("SELECT id, content FROM chunks ORDER BY id")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Update chunk embedding
    pub fn update_embedding(&self, chunk_id: i64, embedding: &[f32]) -> Result<()> {
        let embedding_bytes = embeddings::embedding_to_bytes(embedding);

        self.db.conn.execute(
            "UPDATE chunks SET embedding = ?1 WHERE id = ?2",
            params![embedding_bytes, chunk_id],
        )?;

        Ok(())
    }
}

/// Build a safe FTS5 `MATCH` expression from a raw query.
///
/// Each whitespace token (len ≥ 2) is stripped of double quotes, wrapped as a
/// double-quoted FTS5 string literal, and the literals are OR-joined so any term
/// can match. Quoting each token means punctuation such as `0.3`, `?`, or `-` is
/// treated as literal text rather than FTS5 operator syntax. Returns `None` when
/// no usable term remains.
fn fts_match_expr(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '-'))
        .filter(|w| w.chars().count() >= 2)
        .map(|w| format!("\"{}\"", w.replace('"', "")))
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" OR "))
    }
}

#[cfg(test)]
mod tests {
    use super::fts_match_expr;
    use crate::storage::{ChunkStore, Database, DocumentStore};

    /// A hermetic on-disk SQLite DB in a fresh temp dir (dropped with the guard).
    fn temp_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_at_path(dir.path().join("test.db")).unwrap();
        (dir, db)
    }

    fn seed_doc(db: &Database, filename: &str) -> i64 {
        DocumentStore::new(db)
            .insert("/tmp/src", filename, "text", "full content", None)
            .unwrap()
    }

    #[test]
    fn fts_search_ranks_keyword_hits_and_syncs_on_insert() {
        let (_dir, db) = temp_db();
        let doc_id = seed_doc(&db, "bio.md");
        let store = ChunkStore::new(&db);
        store.init_schema().unwrap();

        store
            .insert(
                doc_id,
                0,
                "The mitochondria is the powerhouse of the cell",
                None,
            )
            .unwrap();
        store
            .insert(
                doc_id,
                1,
                "Photosynthesis converts light into chemical energy",
                None,
            )
            .unwrap();
        store
            .insert(doc_id, 2, "Unrelated text about tectonic plates", None)
            .unwrap();

        // The insert trigger keeps chunks_fts in sync — a MATCH finds the row.
        let hits = store.search_content_fts("mitochondria", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chunk_index, 0);
        assert!(hits[0].content.contains("mitochondria"));
    }

    #[test]
    fn fts_backfills_chunks_that_predate_the_index() {
        let (_dir, db) = temp_db();
        let doc_id = seed_doc(&db, "legacy.md");

        // Simulate a pre-B-001 database: a chunks table with rows but no FTS.
        db.conn
            .execute(
                "CREATE TABLE chunks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    document_id INTEGER NOT NULL,
                    chunk_index INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    embedding BLOB
                )",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO chunks (document_id, chunk_index, content) VALUES (?1, 0, ?2)",
                rusqlite::params![doc_id, "legacy content about enzymes"],
            )
            .unwrap();

        // init_schema must create the index and rebuild it from existing rows.
        let store = ChunkStore::new(&db);
        store.init_schema().unwrap();

        let hits = store.search_content_fts("enzymes", 10).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn hybrid_search_returns_structured_citations() {
        let (_dir, db) = temp_db();
        let doc_id = seed_doc(&db, "cells.md");
        let store = ChunkStore::new(&db);
        store.init_schema().unwrap();
        // No embeddings → semantic arm is skipped (hermetic; no model load).
        store
            .insert(doc_id, 4, "Ribosomes synthesize proteins in the cell", None)
            .unwrap();

        let doc_store = DocumentStore::new(&db);
        let hits = crate::retrieval::hybrid_search(&store, &doc_store, "ribosomes", 5).unwrap();

        assert_eq!(hits.len(), 1);
        let hit = &hits[0];
        assert_eq!(hit.document_id, doc_id);
        assert_eq!(hit.chunk_index, 4);
        assert_eq!(hit.filename, "cells.md");
        assert!(hit.content.contains("Ribosomes"));
    }

    #[test]
    fn fts_match_expr_quotes_and_or_joins() {
        assert_eq!(
            fts_match_expr("photosynthesis process"),
            Some("\"photosynthesis\" OR \"process\"".to_string())
        );
    }

    #[test]
    fn fts_match_expr_keeps_numeric_references() {
        // A reference like "0.3" must survive as a literal term.
        let expr = fts_match_expr("exercise 0.3").unwrap();
        assert!(expr.contains("\"0.3\""));
        assert!(expr.contains("\"exercise\""));
    }

    #[test]
    fn fts_match_expr_strips_punctuation_noise() {
        // Trailing "?" and stray quotes must not reach FTS5 as operators.
        let expr = fts_match_expr("mitochondria?").unwrap();
        assert_eq!(expr, "\"mitochondria\"");
        let expr = fts_match_expr("a \"quoted\" term").unwrap();
        assert_eq!(expr, "\"quoted\" OR \"term\"");
    }

    #[test]
    fn fts_match_expr_empty_when_no_terms() {
        assert_eq!(fts_match_expr("a ? !"), None);
    }
}
