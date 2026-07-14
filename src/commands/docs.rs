//! Headless document commands (`list`, `search <query>`, `delete <id>`).
//!
//! Interactive document management moved to the ratatui TUI Docs pane at E3
//! Phase 3; these functions stay for scripting and never prompt.

use anyhow::Result;
use colored::Colorize;

use crate::storage::{Database, Document, DocumentStore};

/// List all documents
pub async fn list() -> Result<()> {
    let db = Database::open()?;
    let store = DocumentStore::new(&db);

    let documents = store.list()?;

    if documents.is_empty() {
        println!("{}", "No documents found.".dimmed());
        println!("Use {} to add content.", "librarian add".cyan());
        return Ok(());
    }

    println!("\n{} ({} documents)\n", "Documents".bold(), documents.len());

    for doc in &documents {
        print_document_summary(doc);
    }

    Ok(())
}

/// Search documents. A bare `search` (no query) opens the TUI Search pane, so
/// this headless path only runs with an explicit query.
pub async fn search(query: Option<String>) -> Result<()> {
    let query = match query {
        Some(q) => q,
        None => {
            println!(
                "{} Provide a query: {}",
                "Note:".yellow(),
                "librarian search <query>".cyan()
            );
            return Ok(());
        }
    };

    if query.trim().is_empty() {
        println!("{}", "Empty query.".dimmed());
        return Ok(());
    }

    let db = Database::open()?;
    let store = DocumentStore::new(&db);

    let documents = store.search(&query)?;

    if documents.is_empty() {
        println!("{} No documents found for '{}'", "⊘".yellow(), query);
        return Ok(());
    }

    println!(
        "\n{} {} results for '{}'\n",
        "Search:".bold(),
        documents.len(),
        query.cyan()
    );

    for doc in &documents {
        print_document_summary(doc);
    }

    Ok(())
}

/// Delete a document by id (headless). The caller passed an explicit id, so the
/// deletion happens directly without a confirmation prompt.
pub async fn delete(id: Option<i64>) -> Result<()> {
    let id = match id {
        Some(id) => id,
        None => {
            println!(
                "{} Provide a document id: {}",
                "Note:".yellow(),
                "librarian delete <id>".cyan()
            );
            return Ok(());
        }
    };

    let db = Database::open()?;
    let store = DocumentStore::new(&db);

    match store.get(id)? {
        Some(doc) => {
            if store.delete(id)? {
                println!(
                    "{} Deleted document {} '{}' ({})",
                    "✓".green(),
                    id,
                    doc.filename,
                    doc.content_type
                );
            } else {
                println!("{} Failed to delete document {}", "✗".red(), id);
            }
        }
        None => {
            println!("{} Document not found: {}", "✗".red(), id);
        }
    }

    Ok(())
}

fn print_document_summary(doc: &Document) {
    let tags = doc.tags.as_deref().unwrap_or("");
    let tags_display = if tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", tags.cyan())
    };

    println!(
        "  {} {} {}{} ({} chars)",
        format!("[{}]", doc.id).dimmed(),
        doc.filename.bold(),
        doc.content_type.dimmed(),
        tags_display,
        doc.content.len()
    );
}
