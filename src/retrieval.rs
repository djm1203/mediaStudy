//! Unified hybrid retrieval.
//!
//! Two arms run over the current bucket's chunks:
//! - **semantic** — cosine similarity over local embeddings
//!   ([`embeddings::find_similar`]),
//! - **keyword** — the `chunks_fts` FTS5 index ([`ChunkStore::search_content_fts`]),
//!   falling back to the legacy `LIKE` arm when FTS is unavailable.
//!
//! Their ranked outputs are merged with **Reciprocal Rank Fusion** (B-012)
//! instead of the old keyword-then-semantic concatenation, and every returned
//! chunk carries the `document_id` + `chunk_index` + `filename` needed for
//! verifiable, browsable citations (B-011).

use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::Result;

use crate::embeddings;
use crate::search::chunks_overlap;
use crate::storage::{ChunkStore, DocumentStore};

/// A chunk selected by hybrid retrieval, with everything needed to cite it.
#[derive(Debug, Clone)]
pub struct RetrievedChunk {
    #[allow(dead_code)]
    pub chunk_id: i64,
    pub document_id: i64,
    pub chunk_index: i64,
    pub filename: String,
    pub content: String,
    /// Fused Reciprocal Rank Fusion score (higher = more relevant).
    #[allow(dead_code)]
    pub score: f32,
}

/// A verifiable citation carried alongside a generated answer (B-011).
///
/// `marker` is the 1-based number that appears as `[Source N]` in the context
/// block handed to the model, so a reader (or the TUI sources view) can map an
/// inline `[Source N]` back to a concrete document + chunk.
#[derive(Debug, Clone)]
pub struct Citation {
    pub marker: usize,
    /// Carried for a future "open the cited document" jump; part of the
    /// verifiable-citation contract even though the current sources view keys
    /// off `filename` + `chunk_index`.
    #[allow(dead_code)]
    pub document_id: i64,
    pub chunk_index: i64,
    pub filename: String,
}

/// RRF constant. 60 is the value from the original Cormack et al. paper and the
/// common default; it damps the contribution of low-ranked items.
const RRF_K: f32 = 60.0;

/// Run hybrid retrieval for `query`, returning up to `top_k` chunks ranked by
/// Reciprocal Rank Fusion of the semantic + keyword arms, deduplicated by
/// content overlap.
pub fn hybrid_search(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    query: &str,
    top_k: usize,
) -> Result<Vec<RetrievedChunk>> {
    // Pull more than top_k from each arm so fusion has material to work with.
    let arm_k = (top_k * 2).max(10);

    // --- Semantic arm: cosine over all embedded chunks ---
    let embedded = chunk_store.get_all_with_embeddings()?;
    let embs: Vec<(i64, Vec<f32>)> = embedded
        .iter()
        .filter_map(|c| c.embedding.as_ref().map(|e| (c.id, e.clone())))
        .collect();
    // Only embed the query when there is something to compare it against —
    // saves loading the model when a bucket has no embedded chunks.
    let semantic_ranked: Vec<i64> = if embs.is_empty() {
        Vec::new()
    } else {
        match embeddings::embed_text(query) {
            Ok(q) => embeddings::find_similar(&q, &embs, arm_k)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
            Err(_) => Vec::new(),
        }
    };

    // --- Keyword arm: FTS5, falling back to LIKE ---
    let keyword_chunks = match chunk_store.search_content_fts(query, arm_k) {
        Ok(v) if !v.is_empty() => v,
        _ => chunk_store.search_content(query, arm_k).unwrap_or_default(),
    };
    let keyword_ranked: Vec<i64> = keyword_chunks.iter().map(|c| c.id).collect();

    // --- Reciprocal Rank Fusion ---
    let mut scores: HashMap<i64, f32> = HashMap::new();
    for ranked in [&semantic_ranked, &keyword_ranked] {
        for (rank, id) in ranked.iter().enumerate() {
            *scores.entry(*id).or_default() += 1.0 / (RRF_K + rank as f32 + 1.0);
        }
    }
    if scores.is_empty() {
        return Ok(Vec::new());
    }

    let mut ranked: Vec<(i64, f32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    // Resolve each surviving id to a chunk (from whichever arm loaded it),
    // dedup by content overlap, and cap at top_k.
    let mut out: Vec<RetrievedChunk> = Vec::new();
    let mut filename_cache: HashMap<i64, String> = HashMap::new();

    for (id, score) in ranked {
        if out.len() >= top_k {
            break;
        }
        let Some(c) = embedded
            .iter()
            .find(|c| c.id == id)
            .or_else(|| keyword_chunks.iter().find(|c| c.id == id))
        else {
            continue;
        };
        if out
            .iter()
            .any(|r| chunks_overlap(&r.content, &c.content, 0.8))
        {
            continue;
        }
        let filename = match filename_cache.get(&c.document_id) {
            Some(f) => f.clone(),
            None => {
                let f = doc_store
                    .get(c.document_id)?
                    .map(|d| d.filename)
                    .unwrap_or_else(|| "Unknown".to_string());
                filename_cache.insert(c.document_id, f.clone());
                f
            }
        };
        out.push(RetrievedChunk {
            chunk_id: c.id,
            document_id: c.document_id,
            chunk_index: c.chunk_index,
            filename,
            content: c.content.clone(),
            score,
        });
    }

    Ok(out)
}

/// Format retrieved chunks into a context block whose sources are numbered
/// `[Source N: filename (chunk X)]`, together with the parallel [`Citation`]
/// list. Stops once `max_chars` of budget is consumed.
pub fn build_context(chunks: &[RetrievedChunk], max_chars: usize) -> (String, Vec<Citation>) {
    let mut context = String::new();
    let mut citations = Vec::new();
    let mut total = 0usize;

    for chunk in chunks {
        if total >= max_chars {
            break;
        }
        let marker = citations.len() + 1;
        let remaining = max_chars - total;
        let body = truncate_content(&chunk.content, remaining.min(2000));
        let header = format!(
            "[Source {}: {} (chunk {})]",
            marker, chunk.filename, chunk.chunk_index
        );
        context.push_str(&header);
        context.push('\n');
        context.push_str(&body);
        context.push_str("\n\n");
        total += body.len() + header.len() + 2;
        citations.push(Citation {
            marker,
            document_id: chunk.document_id,
            chunk_index: chunk.chunk_index,
            filename: chunk.filename.clone(),
        });
    }

    (context, citations)
}

/// Truncate content to `max_len`, preferring a sentence/paragraph boundary.
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }
    let boundary = (0..=max_len)
        .rev()
        .find(|&i| content.is_char_boundary(i))
        .unwrap_or(0);
    let truncated = &content[..boundary];
    if let Some(pos) = truncated.rfind(". ") {
        return format!("{}.", &truncated[..pos]);
    }
    if let Some(pos) = truncated.rfind('\n') {
        return truncated[..pos].to_string();
    }
    format!("{}...", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: i64, doc: i64, idx: i64, content: &str) -> RetrievedChunk {
        RetrievedChunk {
            chunk_id: id,
            document_id: doc,
            chunk_index: idx,
            filename: format!("doc{doc}.md"),
            content: content.to_string(),
            score: 0.0,
        }
    }

    #[test]
    fn build_context_numbers_sources_1_based() {
        let chunks = vec![chunk(1, 7, 3, "alpha"), chunk(2, 8, 0, "beta")];
        let (ctx, cites) = build_context(&chunks, 8000);
        assert!(ctx.contains("[Source 1: doc7.md (chunk 3)]"));
        assert!(ctx.contains("[Source 2: doc8.md (chunk 0)]"));
        assert_eq!(cites.len(), 2);
        assert_eq!(cites[0].marker, 1);
        assert_eq!(cites[0].document_id, 7);
        assert_eq!(cites[0].chunk_index, 3);
        assert_eq!(cites[1].marker, 2);
        assert_eq!(cites[1].document_id, 8);
    }

    #[test]
    fn build_context_respects_char_budget() {
        let big = "x".repeat(5000);
        let chunks = vec![chunk(1, 1, 0, &big), chunk(2, 2, 0, &big)];
        // Budget only fits the first (truncated) source.
        let (_, cites) = build_context(&chunks, 1500);
        assert_eq!(cites.len(), 1);
    }

    #[test]
    fn truncate_content_prefers_sentence_boundary() {
        let text = "First sentence. Second sentence that runs on and on and on.";
        let out = truncate_content(text, 25);
        assert_eq!(out, "First sentence.");
    }
}
