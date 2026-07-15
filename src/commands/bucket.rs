//! Headless bucket (library) commands.
//!
//! Interactive library management moved to the ratatui TUI (sidebar + Home) at
//! E3 Phase 3. These functions stay for scripting and never prompt: `create`
//! auto-switches to the new bucket, and `delete` removes it directly.

use anyhow::Result;
use colored::Colorize;

use crate::bucket::{self, Bucket};
use crate::storage::{Database, DocumentStore};

/// Create a new bucket and switch to it. A bare `bucket create` (no name) opens
/// the TUI instead, so this headless path always receives a name.
pub async fn create(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            println!(
                "{} Provide a name: {}",
                "Note:".yellow(),
                "librarian bucket create <name>".cyan()
            );
            return Ok(());
        }
    };

    if name.trim().is_empty() {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    }

    match Bucket::create(&name) {
        Ok(bucket) => {
            println!("{} Created bucket '{}'", "✓".green(), bucket.name);

            // Auto-switch to the newly created bucket.
            bucket::set_current_bucket(Some(&bucket.name))?;
            println!("{} Now using bucket '{}'", "✓".green(), bucket.name);
        }
        Err(e) => {
            println!("{} {}", "✗".red(), e);
        }
    }

    Ok(())
}

/// List all buckets
pub async fn list() -> Result<()> {
    let buckets = Bucket::list_all()?;
    let current = bucket::get_current_bucket()?;
    let current_name = current.as_ref().map(|b| b.name.as_str());

    if buckets.is_empty() {
        println!("{}", "No buckets found.".dimmed());
        println!("Create one with {}", "librarian bucket create".cyan());
        return Ok(());
    }

    println!("\n{}\n", "Buckets:".bold());

    for name in &buckets {
        let is_current = current_name == Some(name.as_str());
        let marker = if is_current {
            "→ ".green()
        } else {
            "  ".normal()
        };
        let suffix = if is_current {
            " (current)".green().to_string()
        } else {
            String::new()
        };

        // Get document count for this bucket
        let bucket = Bucket::open(name)?;
        let db = Database::open_for_bucket(&bucket)?;
        let store = DocumentStore::new(&db);
        let count = store.count()?;

        println!("{}{}{}  ({} documents)", marker, name.bold(), suffix, count);
    }

    Ok(())
}

/// Switch to a different bucket. A bare `bucket use` (no name) opens the TUI, so
/// this headless path always receives a name.
pub async fn switch(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            println!(
                "{} Provide a name: {}",
                "Note:".yellow(),
                "librarian bucket use <name>".cyan()
            );
            return Ok(());
        }
    };

    if !Bucket::exists(&name)? {
        println!("{} Bucket '{}' does not exist", "✗".red(), name);
        return Ok(());
    }

    bucket::set_current_bucket(Some(&name))?;
    println!("{} Now using bucket '{}'", "✓".green(), name);

    Ok(())
}

/// Delete a bucket and all its documents (headless). The caller passed an
/// explicit name, so the deletion happens directly without a confirmation
/// prompt.
pub async fn delete(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            println!(
                "{} Provide a name: {}",
                "Note:".yellow(),
                "librarian bucket delete <name>".cyan()
            );
            return Ok(());
        }
    };

    if !Bucket::exists(&name)? {
        println!("{} Bucket '{}' does not exist", "✗".red(), name);
        return Ok(());
    }

    // Document count (for the confirmation message). Scoped so the database
    // connection is dropped before we remove the directory — otherwise Windows
    // refuses to delete the still-open `documents.db` (os error 32).
    let count = {
        let bucket = Bucket::open(&name)?;
        let db = Database::open_for_bucket(&bucket)?;
        DocumentStore::new(&db).count()?
    };

    // Clear current bucket if this was it.
    let current = bucket::get_current_bucket()?;
    if current.as_ref().map(|b| b.name.as_str()) == Some(&name) {
        bucket::set_current_bucket(None)?;
    }

    Bucket::delete(&name)?;
    println!(
        "{} Deleted bucket '{}' ({} documents)",
        "✓".green(),
        name,
        count
    );

    Ok(())
}

/// Show current bucket status (for use in other commands)
pub fn print_bucket_context() {
    match bucket::get_current_bucket() {
        Ok(Some(bucket)) => {
            println!("{} {}", "Bucket:".dimmed(), bucket.name.cyan());
        }
        Ok(None) => {
            println!("{} {}", "Bucket:".dimmed(), "(default)".dimmed());
        }
        Err(_) => {}
    }
}
