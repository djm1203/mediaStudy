//! Chat context builders shared with the TUI Chat pane.
//!
//! The interactive chat loop now lives in the ratatui TUI (`src/tui/`); the
//! legacy prompt-based `run()` was removed at E3 Phase 3. These pure helpers
//! build RAG context from the current bucket and are reused by `tui::service`.

use anyhow::Result;

use crate::retrieval::{self, Citation};
use crate::storage::{ChunkStore, DocumentStore};

/// Build grounded RAG context using unified hybrid retrieval (RRF over the
/// semantic + keyword arms), returning the context block and the parallel
/// [`Citation`] list carrying `document_id` + `chunk_index` for each numbered
/// `[Source N]` (B-011/B-012).
///
/// Falls back to document-level full-text context (with no structured citations)
/// when hybrid retrieval finds nothing — e.g. a bucket whose chunks have no
/// embeddings yet.
pub fn build_grounded_context(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    query: &str,
    max_context_chars: usize,
) -> Result<(String, Vec<Citation>)> {
    let hits = retrieval::hybrid_search(chunk_store, doc_store, query, 10)?;

    if hits.is_empty() {
        let context = build_fts_context(doc_store, query, max_context_chars)?;
        return Ok((context, Vec::new()));
    }

    Ok(retrieval::build_context(&hits, max_context_chars))
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
