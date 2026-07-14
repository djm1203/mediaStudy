//! Chat context builders shared with the TUI Chat pane.
//!
//! The interactive chat loop now lives in the ratatui TUI (`src/tui/`); the
//! legacy prompt-based `run()` was removed at E3 Phase 3. These pure helpers
//! build RAG context from the current bucket and are reused by `tui::service`.

use anyhow::Result;

use crate::embeddings;
use crate::storage::{ChunkStore, DocumentStore};

/// Build context using hybrid search: semantic (embeddings) + keyword (LIKE) combined
pub fn build_semantic_context(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    query: &str,
    max_context_chars: usize,
) -> Result<String> {
    // Get all chunks with embeddings for semantic search
    let chunks = chunk_store.get_all_with_embeddings()?;

    if chunks.is_empty() {
        return build_fts_context(doc_store, query, max_context_chars);
    }

    // --- Semantic search: find top 10 similar chunks ---
    let semantic_ids: Vec<i64> = match embeddings::embed_text(query) {
        Ok(query_embedding) => {
            let chunk_embeddings: Vec<(i64, Vec<f32>)> = chunks
                .iter()
                .filter_map(|c| c.embedding.as_ref().map(|e| (c.id, e.clone())))
                .collect();
            let similar = embeddings::find_similar(&query_embedding, &chunk_embeddings, 10);
            similar.iter().map(|(id, _)| *id).collect()
        }
        Err(_) => Vec::new(),
    };

    // --- Keyword search: find chunks containing query terms ---
    let keyword_chunks = chunk_store.search_content(query, 10).unwrap_or_default();
    let keyword_ids: Vec<i64> = keyword_chunks.iter().map(|c| c.id).collect();

    // --- Merge results: keyword hits first (more precise), then semantic ---
    let mut seen = std::collections::HashSet::new();
    let mut merged_ids: Vec<i64> = Vec::new();

    // Keyword results are more precise for specific references (exercise 0.3, page 26, etc.)
    for id in &keyword_ids {
        if seen.insert(*id) {
            merged_ids.push(*id);
        }
    }
    // Then semantic results
    for id in &semantic_ids {
        if seen.insert(*id) {
            merged_ids.push(*id);
        }
    }

    if merged_ids.is_empty() {
        return build_fts_context(doc_store, query, max_context_chars);
    }

    // Collect matched chunks for dedup — from both the loaded chunks and keyword results
    let mut matched_chunks: Vec<(i64, String)> = Vec::new();
    for id in &merged_ids {
        // Try loaded chunks first
        if let Some(c) = chunks.iter().find(|c| c.id == *id) {
            matched_chunks.push((c.id, c.content.clone()));
        } else if let Some(c) = keyword_chunks.iter().find(|c| c.id == *id) {
            matched_chunks.push((c.id, c.content.clone()));
        }
    }

    // Deduplicate chunks with overlapping content
    let deduped = crate::search::deduplicate_chunks(matched_chunks);

    // Build context from deduped chunks
    let mut context = String::new();
    let mut total_chars = 0;

    for (chunk_id, content) in &deduped {
        if total_chars >= max_context_chars {
            break;
        }

        // Find original chunk for metadata — check both sources
        let chunk = chunks.iter().find(|c| c.id == *chunk_id);
        let kw_chunk = keyword_chunks.iter().find(|c| c.id == *chunk_id);
        let (doc_id, chunk_idx) = chunk
            .or(kw_chunk)
            .map(|c| (c.document_id, c.chunk_index))
            .unwrap_or((0, 0));

        let doc = doc_store.get(doc_id)?;
        let filename = doc
            .map(|d| d.filename)
            .unwrap_or_else(|| "Unknown".to_string());

        let remaining = max_context_chars - total_chars;
        let truncated = truncate_content(content, remaining.min(2000));

        context.push_str(&format!(
            "--- Document: {} (chunk {}) ---\n{}\n\n",
            filename, chunk_idx, truncated
        ));

        total_chars += truncated.len() + filename.len() + 50;
    }

    Ok(context)
}

/// Build context using full-text search (fallback) with dynamic sizing
pub fn build_fts_context(
    store: &DocumentStore,
    query: &str,
    max_context_chars: usize,
) -> Result<String> {
    let results = store.search(query)?;

    if results.is_empty() {
        let all_docs = store.list()?;
        if all_docs.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::new();
        for doc in all_docs.iter().take(3) {
            let preview = truncate_content(&doc.content, 1500);
            context.push_str(&format!(
                "--- Document: {} ---\n{}\n\n",
                doc.filename, preview
            ));
        }
        return Ok(context);
    }

    let mut context = String::new();
    let mut total_chars = 0;

    for doc in results.iter().take(5) {
        if total_chars >= max_context_chars {
            break;
        }

        let remaining = max_context_chars - total_chars;
        let preview = truncate_content(&doc.content, remaining.min(2000));

        context.push_str(&format!(
            "--- Document: {} ---\n{}\n\n",
            doc.filename, preview
        ));

        total_chars += preview.len() + doc.filename.len() + 30;
    }

    Ok(context)
}

/// Truncate content to a maximum length, trying to break at sentence boundaries
pub fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    let truncated = &content[..max_len];

    if let Some(pos) = truncated.rfind(". ") {
        return format!("{}.", &truncated[..pos]);
    }

    if let Some(pos) = truncated.rfind("\n\n") {
        return truncated[..pos].to_string();
    }

    if let Some(pos) = truncated.rfind('\n') {
        return truncated[..pos].to_string();
    }

    format!("{}...", truncated)
}
