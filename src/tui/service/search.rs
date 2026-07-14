//! Search pane service (blueprint §4).
//!
//! Calls `DocumentStore::search` (FTS over the current bucket) inside
//! `tokio::task::spawn_blocking`, mapping each [`Document`] to a [`SearchHit`]
//! with a short excerpt built around the query.
//!
//! [`Document`]: crate::storage::documents::Document

use anyhow::Result;
use tokio::task;

use crate::storage::documents::Document;
use crate::storage::{Database, DocumentStore};
use crate::tui::action::{Message, SearchHit};

/// Run a full-text search and return ranked [`SearchHit`]s as
/// [`Message::SearchResults`]. Opens its own [`Database`] on a blocking
/// thread; never holds a connection across `.await`.
pub async fn run_search(query: String) -> Result<Message> {
    let q = query.clone();
    let hits = task::spawn_blocking(move || -> Result<Vec<SearchHit>> {
        let db = Database::open()?;
        let store = DocumentStore::new(&db);
        let docs = store.search(&q)?;
        Ok(docs.into_iter().map(|doc| to_hit(doc, &q)).collect())
    })
    .await??;

    Ok(Message::SearchResults { query, hits })
}

/// Map a [`Document`] to a [`SearchHit`], building a short snippet around the
/// first query term found in the content.
fn to_hit(doc: Document, query: &str) -> SearchHit {
    let snippet = build_snippet(&doc.content, query);
    SearchHit {
        id: doc.id,
        filename: doc.filename,
        content_type: doc.content_type,
        snippet,
    }
}

/// How many characters of context to keep on each side of a matched term.
const SNIPPET_RADIUS: usize = 80;
/// Fallback excerpt length when no query term is found in the content.
const SNIPPET_MAX: usize = 160;

/// Build a short, single-line excerpt from `content` centered on the first
/// occurrence of a query term (case-insensitive), or a leading excerpt if no
/// term is found in the content at all.
fn build_snippet(content: &str, query: &str) -> String {
    // Collapse whitespace/newlines so the snippet renders on one line.
    let flat: String = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return String::new();
    }

    let lower = flat.to_lowercase();
    let term = query
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .find(|w| !w.is_empty())
        .unwrap_or("")
        .to_lowercase();

    let found = if term.is_empty() {
        None
    } else {
        lower.find(&term)
    };

    match found {
        Some(byte_idx) => {
            let start = floor_char_boundary(&flat, byte_idx.saturating_sub(SNIPPET_RADIUS));
            let end = ceil_char_boundary(
                &flat,
                (byte_idx + term.len() + SNIPPET_RADIUS).min(flat.len()),
            );
            let mut snippet = flat[start..end].to_string();
            if start > 0 {
                snippet = format!("…{snippet}");
            }
            if end < flat.len() {
                snippet.push('…');
            }
            snippet
        }
        None => {
            let end = ceil_char_boundary(&flat, SNIPPET_MAX.min(flat.len()));
            let mut snippet = flat[..end].to_string();
            if end < flat.len() {
                snippet.push('…');
            }
            snippet
        }
    }
}

/// Walk back to the nearest `char` boundary at or before `idx`.
fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Walk forward to the nearest `char` boundary at or after `idx`.
fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}
