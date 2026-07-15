//! Add (ingest) pane service (blueprint §4 — highest-effort pane).
//!
//! Reimplements the interactive `commands/add.rs:110-425` loop for the TUI:
//! `ingest::{requires_transcription,extract_from_file_async,fetch_url}` →
//! `chunk_text`/`ChunkConfig` → `embeddings::embed_text` →
//! `DocumentStore::{exists_by_path,insert}` → `ChunkStore::{init_schema,insert}`,
//! emitting [`Message::IngestProgress`] / [`Message::IngestFileDone`] as it goes
//! and a terminal [`Message::IngestComplete`].
//!
//! Text extraction (PDF/text parse, media transcription, URL/YouTube fetch) runs
//! on the async runtime; **every `rusqlite` + `fastembed` unit runs inside a
//! `tokio::task::spawn_blocking` closure that opens its own [`Database`]** — a
//! connection is never held across an `.await`, never shared with the render
//! loop. Errors are handled per-file (logged into a message) so one bad file
//! never aborts the run.

use std::path::{Path, PathBuf};

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task;

use crate::embeddings;
use crate::ingest::{self, ChunkConfig, ContentType, chunk_text, fetch_url};
use crate::storage::{ChunkStore, Database, DocumentStore};
use crate::tui::action::{Message, ToastLevel};

/// Ingest a local path or URL into the current bucket, streaming incremental
/// progress messages over `msg_tx` and finishing with
/// [`Message::IngestComplete`].
pub async fn start_ingest(
    source: String,
    is_url: bool,
    msg_tx: UnboundedSender<Message>,
) -> Result<()> {
    // Treat anything with an http(s) scheme as a URL regardless of the toggle,
    // mirroring `commands/add.rs::run`.
    let is_url = is_url || source.starts_with("http://") || source.starts_with("https://");

    let (added, skipped) = if is_url {
        ingest_url(&source, &msg_tx).await
    } else {
        let path = PathBuf::from(&source);
        if !path.exists() {
            log_line(&msg_tx, format!("✗ {source} · path does not exist"));
            (0, 0)
        } else if path.is_dir() {
            ingest_directory(&path, &msg_tx).await
        } else {
            ingest_file(&path, &msg_tx).await
        }
    };

    let _ = msg_tx.send(Message::IngestComplete { added, skipped });
    Ok(())
}

// ---------------------------------------------------------------------------
// Single file
// ---------------------------------------------------------------------------

/// Ingest one local file. Returns `(added, skipped)` (each 0 or 1).
async fn ingest_file(path: &Path, msg_tx: &UnboundedSender<Message>) -> (usize, usize) {
    let filename = file_name(path);
    let _ = msg_tx.send(Message::IngestProgress {
        done: 0,
        total: 1,
        current: filename.clone(),
    });

    let source_path = match tokio::fs::canonicalize(path).await {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            log_line(msg_tx, format!("✗ {filename} · {e}"));
            return (0, 0);
        }
    };

    match exists_by_path(source_path.clone()).await {
        Ok(true) => {
            log_line(msg_tx, format!("⊘ {filename} · already in library"));
            let _ = msg_tx.send(Message::IngestProgress {
                done: 1,
                total: 1,
                current: String::new(),
            });
            return (0, 1);
        }
        Ok(false) => {}
        Err(e) => {
            log_line(msg_tx, format!("✗ {filename} · {e}"));
            return (0, 0);
        }
    }

    if ingest::requires_transcription(path) {
        let _ = msg_tx.send(Message::IngestProgress {
            done: 0,
            total: 1,
            current: format!("Transcribing {filename}…"),
        });
    }

    // Extraction (transcription/OCR/parse) runs on the async runtime.
    let content = match ingest::extract_from_file_async(path).await {
        Ok(c) => c,
        Err(e) => {
            log_line(msg_tx, format!("✗ {filename} · {e}"));
            return (0, 0);
        }
    };

    // Content-based dedup (B-019): identical content under any path is skipped.
    if let Ok(true) = exists_by_content(content.text.clone()).await {
        log_line(msg_tx, format!("⊘ {filename} · duplicate content"));
        let _ = msg_tx.send(Message::IngestProgress {
            done: 1,
            total: 1,
            current: String::new(),
        });
        return (0, 1);
    }

    let content_type = content_type_str(&content.content_type).to_string();
    match embed_and_store(
        source_path,
        filename.clone(),
        content_type,
        content.text,
        Some(msg_tx.clone()),
    )
    .await
    {
        Ok(chunks) => {
            let _ = msg_tx.send(Message::IngestFileDone { filename, chunks });
            (1, 0)
        }
        Err(e) => {
            log_line(msg_tx, format!("✗ {filename} · {e}"));
            (0, 0)
        }
    }
}

// ---------------------------------------------------------------------------
// Directory
// ---------------------------------------------------------------------------

/// Ingest every regular file directly inside `dir`. Returns `(added, skipped)`.
async fn ingest_directory(dir: &Path, msg_tx: &UnboundedSender<Message>) -> (usize, usize) {
    let files = match collect_files(dir).await {
        Ok(f) => f,
        Err(e) => {
            log_line(msg_tx, format!("✗ {} · {e}", dir.display()));
            return (0, 0);
        }
    };

    if files.is_empty() {
        log_line(msg_tx, "⊘ no files found in directory".to_string());
        return (0, 0);
    }

    let total = files.len();
    let mut added = 0usize;
    let mut skipped = 0usize;

    for (idx, file_path) in files.iter().enumerate() {
        let filename = file_name(file_path);
        let _ = msg_tx.send(Message::IngestProgress {
            done: idx,
            total,
            current: filename.clone(),
        });

        let source_path = match tokio::fs::canonicalize(file_path).await {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                log_line(msg_tx, format!("✗ {filename} · {e}"));
                continue;
            }
        };

        match exists_by_path(source_path.clone()).await {
            Ok(true) => {
                log_line(msg_tx, format!("⊘ {filename} · already in library"));
                skipped += 1;
                continue;
            }
            Ok(false) => {}
            Err(e) => {
                log_line(msg_tx, format!("✗ {filename} · {e}"));
                continue;
            }
        }

        let content = match ingest::extract_from_file_async(file_path).await {
            Ok(c) => c,
            Err(e) => {
                log_line(msg_tx, format!("✗ {filename} · {e}"));
                continue;
            }
        };

        // Content-based dedup (B-019): identical content under any path.
        if let Ok(true) = exists_by_content(content.text.clone()).await {
            log_line(msg_tx, format!("⊘ {filename} · duplicate content"));
            skipped += 1;
            continue;
        }

        let content_type = content_type_str(&content.content_type).to_string();
        // Per-file: no chunk-level progress so the gauge stays file-granular.
        match embed_and_store(
            source_path,
            filename.clone(),
            content_type,
            content.text,
            None,
        )
        .await
        {
            Ok(chunks) => {
                let _ = msg_tx.send(Message::IngestFileDone { filename, chunks });
                added += 1;
            }
            Err(e) => {
                log_line(msg_tx, format!("✗ {filename} · {e}"));
            }
        }
    }

    let _ = msg_tx.send(Message::IngestProgress {
        done: total,
        total,
        current: String::new(),
    });
    (added, skipped)
}

// ---------------------------------------------------------------------------
// URL / YouTube
// ---------------------------------------------------------------------------

/// Ingest a single URL (article or YouTube transcript). Returns `(added, skipped)`.
async fn ingest_url(url: &str, msg_tx: &UnboundedSender<Message>) -> (usize, usize) {
    let _ = msg_tx.send(Message::IngestProgress {
        done: 0,
        total: 1,
        current: format!("Fetching {url}…"),
    });

    match exists_by_path(url.to_string()).await {
        Ok(true) => {
            log_line(msg_tx, format!("⊘ {url} · already in library"));
            let _ = msg_tx.send(Message::IngestProgress {
                done: 1,
                total: 1,
                current: String::new(),
            });
            return (0, 1);
        }
        Ok(false) => {}
        Err(e) => {
            log_line(msg_tx, format!("✗ {url} · {e}"));
            return (0, 0);
        }
    }

    let content = match fetch_url(url).await {
        Ok(c) => c,
        Err(e) => {
            log_line(msg_tx, format!("✗ {url} · {e}"));
            return (0, 0);
        }
    };

    // Content-based dedup (B-019): skip if identical content is already stored.
    if let Ok(true) = exists_by_content(content.text.clone()).await {
        log_line(msg_tx, format!("⊘ {url} · duplicate content"));
        let _ = msg_tx.send(Message::IngestProgress {
            done: 1,
            total: 1,
            current: String::new(),
        });
        return (0, 1);
    }

    let is_youtube = url.contains("youtube.com") || url.contains("youtu.be");
    let content_type = if is_youtube { "youtube" } else { "url" }.to_string();
    let title = content.title.clone();

    match embed_and_store(
        url.to_string(),
        title.clone(),
        content_type,
        content.text,
        Some(msg_tx.clone()),
    )
    .await
    {
        Ok(chunks) => {
            let _ = msg_tx.send(Message::IngestFileDone {
                filename: title,
                chunks,
            });
            (1, 0)
        }
        Err(e) => {
            log_line(msg_tx, format!("✗ {title} · {e}"));
            (0, 0)
        }
    }
}

// ---------------------------------------------------------------------------
// Blocking DB + embedding unit
// ---------------------------------------------------------------------------

/// Insert the document, chunk it, embed each chunk, and store the chunks — all
/// on a blocking thread that owns its own [`Database`]. When `progress` is set,
/// emits a [`Message::IngestProgress`] after each chunk so a single large file
/// still animates. Returns the number of chunks written.
async fn embed_and_store(
    source_path: String,
    filename: String,
    content_type: String,
    text: String,
    progress: Option<UnboundedSender<Message>>,
) -> Result<usize> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        let doc_store = DocumentStore::new(&db);
        let chunk_store = ChunkStore::new(&db);
        chunk_store.init_schema()?;

        let doc_id = doc_store.insert(&source_path, &filename, &content_type, &text, None)?;

        let chunks = chunk_text(&text, &ChunkConfig::default());
        let total = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            let embedding = embeddings::embed_text(&chunk.text).ok();
            chunk_store.insert(
                doc_id,
                chunk.index as i64,
                &chunk.text,
                embedding.as_deref(),
            )?;
            if let Some(tx) = &progress {
                let _ = tx.send(Message::IngestProgress {
                    done: i + 1,
                    total: total.max(1),
                    current: format!("Embedding {filename} ({}/{})", i + 1, total),
                });
            }
        }

        Ok(total)
    })
    .await?
}

/// Check whether a document with `source_path` already exists, on a blocking
/// thread that owns its own [`Database`].
async fn exists_by_path(source_path: String) -> Result<bool> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        DocumentStore::new(&db).exists_by_path(&source_path)
    })
    .await?
}

/// Check whether byte-identical content already exists (B-019), on a blocking
/// thread that owns its own [`Database`].
async fn exists_by_content(text: String) -> Result<bool> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        DocumentStore::new(&db).exists_by_content(&text)
    })
    .await?
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect the regular files directly inside `dir` (non-recursive), matching the
/// behaviour of `commands/add.rs::process_directory`.
async fn collect_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_path = entry.path();
        // Skip entries we can't stat rather than aborting the whole batch.
        if let Ok(metadata) = tokio::fs::metadata(&file_path).await
            && metadata.is_file()
        {
            files.push(file_path);
        }
    }
    files.sort();
    Ok(files)
}

/// Short display name for a path.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// Map a [`ContentType`] to the string stored in `documents.content_type`
/// (reimplements the private helper in `commands/add.rs`).
fn content_type_str(ct: &ContentType) -> &'static str {
    match ct {
        ContentType::Pdf => "pdf",
        ContentType::Text => "text",
        ContentType::Markdown => "markdown",
        ContentType::Audio => "audio",
        ContentType::Video => "video",
        ContentType::Image => "image",
        ContentType::Url => "url",
        ContentType::Unknown => "unknown",
    }
}

/// Push a pre-formatted, glyph-prefixed status line into the pane's log by
/// piggy-backing on [`Message::IngestFileDone`] with a `chunks` count of 0 (the
/// pane renders such messages verbatim). Also surfaces a transient toast for
/// hard errors so they are not missed if the log has scrolled.
fn log_line(msg_tx: &UnboundedSender<Message>, line: String) {
    if line.starts_with('✗') {
        let _ = msg_tx.send(Message::Toast {
            level: ToastLevel::Error,
            text: line.trim_start_matches('✗').trim().to_string(),
        });
    }
    let _ = msg_tx.send(Message::IngestFileDone {
        filename: line,
        chunks: 0,
    });
}
