use anyhow::Result;
use colored::Colorize;
use inquire::{Select, Text};

use crate::bucket::{self, Bucket};
use crate::storage::{Database, DocumentStore};

/// Interactive bucket management
pub async fn run() -> Result<()> {
    println!("{}", "Bucket Management".bold().cyan());
    println!("{}", "─".repeat(40).dimmed());

    show_current_bucket();

    let options = vec![
        "Create new bucket",
        "Switch bucket",
        "List buckets",
        "Delete bucket",
        "Use no bucket (default)",
        "Back",
    ];

    loop {
        let selection = Select::new("What would you like to do?", options.clone()).prompt()?;

        match selection {
            "Create new bucket" => create_bucket().await?,
            "Switch bucket" => switch_bucket().await?,
            "List buckets" => list_buckets().await?,
            "Delete bucket" => delete_bucket().await?,
            "Use no bucket (default)" => clear_bucket().await?,
            "Back" => break,
            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}

/// Create a new bucket
pub async fn create(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => Text::new("Bucket name:")
            .with_help_message("e.g., os-class, physics-301, cs-foundations")
            .prompt()?,
    };

    if name.trim().is_empty() {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    }

    match Bucket::create(&name) {
        Ok(bucket) => {
            println!("{} Created bucket '{}'", "✓".green(), bucket.name);

            // Ask if they want to switch to it
            let switch = Select::new(
                "Switch to this bucket now?",
                vec!["Yes (Recommended)", "No"],
            )
            .prompt()?;

            if switch.starts_with("Yes") {
                bucket::set_current_bucket(Some(&bucket.name))?;
                println!("{} Now using bucket '{}'", "✓".green(), bucket.name);
            }
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
        println!("Create one with {}", "media-study bucket create".cyan());
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

/// Switch to a different bucket
pub async fn switch(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            let buckets = Bucket::list_all()?;

            if buckets.is_empty() {
                println!("{}", "No buckets found. Create one first.".dimmed());
                return Ok(());
            }

            Select::new("Select bucket:", buckets).prompt()?
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

/// Delete a bucket
async fn delete_bucket() -> Result<()> {
    let buckets = Bucket::list_all()?;

    if buckets.is_empty() {
        println!("{}", "No buckets to delete.".dimmed());
        return Ok(());
    }

    let name = Select::new("Select bucket to delete:", buckets).prompt()?;

    // Show document count
    let bucket = Bucket::open(&name)?;
    let db = Database::open_for_bucket(&bucket)?;
    let store = DocumentStore::new(&db);
    let count = store.count()?;

    println!(
        "\n{} This bucket contains {} documents.",
        "Warning:".yellow().bold(),
        count
    );

    let confirm = Select::new(
        &format!("Delete bucket '{}' and all its documents?", name),
        vec!["No", "Yes, delete it"],
    )
    .prompt()?;

    if confirm == "Yes, delete it" {
        // Clear current bucket if this was it
        let current = bucket::get_current_bucket()?;
        if current.as_ref().map(|b| b.name.as_str()) == Some(&name) {
            bucket::set_current_bucket(None)?;
        }

        Bucket::delete(&name)?;
        println!("{} Deleted bucket '{}'", "✓".green(), name);
    } else {
        println!("{}", "Cancelled.".dimmed());
    }

    Ok(())
}

async fn create_bucket() -> Result<()> {
    create(None).await
}

async fn switch_bucket() -> Result<()> {
    switch(None).await
}

async fn list_buckets() -> Result<()> {
    list().await
}

async fn clear_bucket() -> Result<()> {
    bucket::set_current_bucket(None)?;
    println!("{} Now using default (no bucket)", "✓".green());
    Ok(())
}

fn show_current_bucket() {
    match bucket::get_current_bucket() {
        Ok(Some(bucket)) => {
            println!("Current bucket: {}\n", bucket.name.cyan().bold());
        }
        Ok(None) => {
            println!("Current bucket: {}\n", "(none - using default)".dimmed());
        }
        Err(_) => {}
    }
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
