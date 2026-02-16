use anyhow::Result;
use colored::Colorize;
use inquire::{Select, Text};

use crate::storage::{Database, Document, DocumentStore};

/// Interactive document management
pub async fn run() -> Result<()> {
    println!();
    println!(
        "    {}",
        "╭──────────────────────────────────────────────────────╮".green()
    );
    println!(
        "    {}          {}          {}",
        "│".green(),
        "📂 DOCUMENT MANAGEMENT 📂".bold().white(),
        "│".green()
    );
    println!(
        "    {}       {}       {}",
        "│".green(),
        "Browse, search, and manage your materials".dimmed(),
        "│".green()
    );
    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".green()
    );
    println!();

    let options = vec![
        "📋  List all documents  │ See everything in this book",
        "🔍  Search documents    │ Find specific content",
        "👁️   View document       │ Read document details",
        "🗑️   Delete document     │ Remove from collection",
        "←   Back",
    ];

    loop {
        let selection = Select::new("What would you like to do?", options.clone()).prompt();

        let selection = match selection {
            Ok(s) => s,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => break,
            Err(e) => return Err(e.into()),
        };

        match selection {
            s if s.contains("List all documents") => {
                if let Err(e) = list().await
                    && !e.to_string().contains("cancelled")
                {
                    eprintln!("{} {}", "Error:".red(), e);
                }
            }
            s if s.contains("Search documents") => {
                if let Err(e) = search(None).await
                    && !e.to_string().contains("cancelled")
                {
                    eprintln!("{} {}", "Error:".red(), e);
                }
            }
            s if s.contains("View document") => {
                if let Err(e) = view_document().await
                    && !e.to_string().contains("cancelled")
                {
                    eprintln!("{} {}", "Error:".red(), e);
                }
            }
            s if s.contains("Delete document") => {
                if let Err(e) = delete_document().await
                    && !e.to_string().contains("cancelled")
                {
                    eprintln!("{} {}", "Error:".red(), e);
                }
            }
            s if s.contains("Back") => break,
            _ => {}
        }

        println!();
    }

    Ok(())
}

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

/// Search documents
pub async fn search(query: Option<String>) -> Result<()> {
    let query = match query {
        Some(q) => q,
        None => Text::new("Search query:")
            .with_help_message("Search document content")
            .prompt()?,
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

/// View a specific document
async fn view_document() -> Result<()> {
    let db = Database::open()?;
    let store = DocumentStore::new(&db);

    let id_str = Text::new("Document ID:")
        .with_help_message("Enter the document ID to view")
        .prompt()?;

    let id: i64 = id_str
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid ID"))?;

    match store.get(id)? {
        Some(doc) => {
            println!("\n{}", "─".repeat(50).dimmed());
            println!("{} {}", "ID:".bold(), doc.id);
            println!("{} {}", "Filename:".bold(), doc.filename);
            println!("{} {}", "Type:".bold(), doc.content_type);
            println!("{} {}", "Source:".bold(), doc.source_path.dimmed());
            println!(
                "{} {}",
                "Tags:".bold(),
                doc.tags.as_deref().unwrap_or("none")
            );
            println!(
                "{} {}",
                "Created:".bold(),
                doc.created_at.format("%Y-%m-%d %H:%M")
            );
            println!("{} {} chars", "Length:".bold(), doc.content.len());
            println!("{}", "─".repeat(50).dimmed());

            // Show content preview or full content
            let preview_len = doc.content.len().min(500);
            println!("\n{}", "Content preview:".bold());
            println!("{}", &doc.content[..preview_len]);
            if doc.content.len() > 500 {
                println!(
                    "{}",
                    format!("... ({} more chars)", doc.content.len() - 500).dimmed()
                );
            }
        }
        None => {
            println!("{} Document not found: {}", "✗".red(), id);
        }
    }

    Ok(())
}

/// Delete a document (public interface)
pub async fn delete(id: Option<i64>) -> Result<()> {
    let db = Database::open()?;
    let store = DocumentStore::new(&db);

    let id = match id {
        Some(id) => id,
        None => {
            let id_str = Text::new("Document ID to delete:")
                .with_help_message("Enter the document ID to delete")
                .prompt()?;
            id_str
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid ID"))?
        }
    };

    // Show document first
    if let Some(doc) = store.get(id)? {
        println!(
            "\n{} {} ({})",
            "Document:".bold(),
            doc.filename,
            doc.content_type
        );

        let confirm = Select::new(
            &format!("Delete document {} '{}'?", id, doc.filename),
            vec!["No", "Yes"],
        )
        .prompt()?;

        if confirm == "Yes" {
            if store.delete(id)? {
                println!("{} Deleted document {}", "✓".green(), id);
            } else {
                println!("{} Failed to delete document {}", "✗".red(), id);
            }
        } else {
            println!("{}", "Cancelled.".dimmed());
        }
    } else {
        println!("{} Document not found: {}", "✗".red(), id);
    }

    Ok(())
}

/// Delete a document (interactive - for menu)
async fn delete_document() -> Result<()> {
    delete(None).await
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
