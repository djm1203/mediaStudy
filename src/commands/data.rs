//! Data-safety & management commands (E5 / B-020, B-021).
//!
//! - `stats`   — per-bucket document/chunk/study counts + on-disk size.
//! - `export`  — write a compacted, portable copy of the current bucket's DB.
//! - `import`  — register an exported `.db` as a new bucket.
//! - `compact` — VACUUM the current bucket in place.
//! - `reembed` — regenerate every chunk embedding with the current model and
//!   record the model identity (safe re-embed when the model changes, B-021).

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::bucket::Bucket;
use crate::embeddings;
use crate::storage::{ChunkStore, Database, maintenance};

/// `librarian stats` — print a summary of the current bucket.
pub fn stats() -> Result<()> {
    let db = Database::open()?;
    let s = maintenance::gather_stats(&db)?;

    println!("\n{}", "Library stats".bold());
    println!("{}", "─".repeat(40).dimmed());
    row("Documents", &s.documents.to_string());
    row("Chunks", &s.chunks.to_string());
    row(
        "Embedded chunks",
        &format!("{} / {}", s.embedded_chunks, s.chunks),
    );
    row("Study items", &s.study_items.to_string());
    row("Conversations", &s.conversations.to_string());
    row("Database size", &format_bytes(s.db_bytes));
    if let Some(model) = db.meta_get("embedding_model")?
        && model != embeddings::MODEL_ID
    {
        println!(
            "\n{} embeddings were built with '{}', current model is '{}'. Run {} to rebuild.",
            "⚠".yellow(),
            model,
            embeddings::MODEL_ID,
            "librarian reembed".cyan()
        );
    }
    println!("{}", "─".repeat(40).dimmed());
    Ok(())
}

/// `librarian export <dest>` — compacted portable copy of the current bucket DB.
pub fn export(dest: String) -> Result<()> {
    let db = Database::open()?;
    let path = std::path::Path::new(&dest);
    maintenance::vacuum_into(&db, path)?;
    let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    println!(
        "{} Exported to {} ({})",
        "✓".green(),
        dest.cyan(),
        format_bytes(bytes)
    );
    Ok(())
}

/// `librarian import <name> <src>` — register an exported `.db` as a new bucket.
pub fn import(name: String, src: String) -> Result<()> {
    let src_path = std::path::Path::new(&src);
    if !src_path.is_file() {
        bail!("source database not found: {}", src);
    }
    validate_librarian_db(src_path)?;

    if Bucket::exists(&name)? {
        bail!("bucket '{}' already exists", name);
    }
    let bucket = Bucket::create(&name)?;
    std::fs::copy(src_path, bucket.db_path())
        .with_context(|| format!("Failed to copy database into bucket '{}'", name))?;

    // Open the imported DB so any pending migrations run, then report.
    let db = Database::open_for_bucket(&bucket)?;
    let s = maintenance::gather_stats(&db)?;
    println!(
        "{} Imported '{}' as bucket {} ({} documents, {} chunks). Switch with {}.",
        "✓".green(),
        src.cyan(),
        bucket.name.cyan(),
        s.documents,
        s.chunks,
        format!("librarian bucket use {}", bucket.name).cyan()
    );
    Ok(())
}

/// `librarian compact` — reclaim free space in the current bucket's DB.
pub fn compact() -> Result<()> {
    let db = Database::open()?;
    let before = std::fs::metadata(&db.path).map(|m| m.len()).unwrap_or(0);
    maintenance::vacuum(&db)?;
    let after = std::fs::metadata(&db.path).map(|m| m.len()).unwrap_or(0);
    println!(
        "{} Compacted {} → {} (saved {})",
        "✓".green(),
        format_bytes(before),
        format_bytes(after),
        format_bytes(before.saturating_sub(after))
    );
    Ok(())
}

/// `librarian reembed` — regenerate every chunk embedding with the current model
/// and stamp the model identity into `meta` (B-021 safe re-embed).
pub fn reembed() -> Result<()> {
    let db = Database::open()?;
    let chunk_store = ChunkStore::new(&db);
    chunk_store.init_schema()?;

    let chunks = chunk_store.get_all_for_reembed()?;
    if chunks.is_empty() {
        println!("{} No chunks to re-embed.", "⊘".yellow());
        return Ok(());
    }

    println!(
        "Re-embedding {} chunks with {}…",
        chunks.len(),
        embeddings::MODEL_ID.cyan()
    );
    let mut done = 0usize;
    for (id, content) in &chunks {
        let embedding = embeddings::embed_text(content)
            .with_context(|| format!("Failed to embed chunk {id}"))?;
        chunk_store.update_embedding(*id, &embedding)?;
        done += 1;
    }

    db.meta_set("embedding_model", embeddings::MODEL_ID)?;
    db.meta_set("embedding_dim", &embeddings::DIM.to_string())?;
    println!("{} Re-embedded {} chunks.", "✓".green(), done);
    Ok(())
}

/// Verify `path` looks like a Librarian database (has a `documents` table)
/// without mutating it — opens a read-only connection.
fn validate_librarian_db(path: &std::path::Path) -> Result<()> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_context(|| format!("Failed to open {}", path.display()))?;
    let has_documents: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='documents'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if has_documents == 0 {
        bail!("{} is not a Librarian database", path.display());
    }
    Ok(())
}

fn row(label: &str, value: &str) {
    println!("  {:<18} {}", format!("{}:", label).dimmed(), value);
}

/// Human-readable byte size (KiB/MiB/GiB).
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn format_bytes_scales_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(format_bytes(1536 * 1024), "1.5 MiB");
    }
}
