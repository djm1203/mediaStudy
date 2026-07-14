use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use colored::Colorize;
use std::io;

mod bucket;
mod commands;
mod config;
mod embeddings;
mod ingest;
mod llm;
mod render;
mod search;
mod storage;
mod tui;

/// ASCII art banner for the application
// Legacy line-based UI — superseded by the ratatui TUI (`src/tui/`); removed at E3 Phase 3.
#[allow(dead_code)]
const BANNER: &str = r#"
    ╔════════════════════════════════════════════════════════╗
    ║                                                        ║
    ║   ▀█▀ █ █ █▀▀   █   █ █▄▄ █▀█ ▄▀█ █▀█ █ ▄▀█ █▄ █     ║
    ║    █  █▀█ ██▄   █▄▄ █ █▄█ █▀▄ █▀█ █▀▄ █ █▀█ █ ▀█     ║
    ║                                                        ║
    ║            ┌─────────────────────────────┐             ║
    ║            │  📚 Your Study Companion 📚  │             ║
    ║            └─────────────────────────────┘             ║
    ╚════════════════════════════════════════════════════════╝
"#;

/// Print the application banner
#[allow(dead_code)]
fn print_banner() {
    println!("{}", BANNER.cyan().bold());
}

/// Print a styled header for a section
#[allow(dead_code)]
fn print_header(title: &str) {
    let width = 50;
    let padding = (width - title.len() - 4) / 2;
    let line = "═".repeat(width);

    println!("\n{}", line.cyan());
    println!(
        "{}{}{}{}{}",
        "║".cyan(),
        " ".repeat(padding),
        title.bold().white(),
        " ".repeat(width - padding - title.len() - 2),
        "║".cyan()
    );
    println!("{}\n", line.cyan());
}

#[derive(Parser)]
#[command(name = "librarian")]
#[command(about = "The Librarian - Your personal AI study companion")]
#[command(
    long_about = "The Librarian helps you study smarter by ingesting your course materials \
(PDFs, videos, audio, notes) and letting you chat with them, generate study guides, \
flashcards, quizzes, and more. Powered by Groq LLM and local embeddings."
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add knowledge (files, directories, URLs, videos)
    Add {
        /// Path or URL to add (skips interactive prompt if provided)
        path: Option<String>,
    },
    /// Ask the Librarian - chat with your materials
    Chat,
    /// Browse your collection
    List,
    /// Search your materials
    Search {
        /// Search query
        query: Option<String>,
    },
    /// Manage documents
    Docs,
    /// Remove a document from your collection
    Delete {
        /// Document ID to delete
        id: Option<i64>,
    },
    /// Manage your library (organize by class/project)
    #[command(alias = "library")]
    Bucket {
        #[command(subcommand)]
        action: Option<BucketAction>,
    },
    /// Configure The Librarian (API keys, model preferences)
    Config,
    /// Study tools - generate guides, flashcards, quizzes
    Generate {
        #[command(subcommand)]
        action: Option<GenerateAction>,
    },
    /// Spaced repetition study session
    Review,
    /// Test your knowledge interactively
    Quiz,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum BucketAction {
    /// Create a new bucket
    Create {
        /// Bucket name
        name: Option<String>,
    },
    /// List all buckets
    List,
    /// Switch to a bucket
    Use {
        /// Bucket name
        name: Option<String>,
    },
    /// Delete a bucket
    Delete {
        /// Bucket name
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum GenerateAction {
    /// Generate a comprehensive study guide
    StudyGuide {
        /// Topic or focus area
        topic: Option<String>,
    },
    /// Generate flashcards for review
    Flashcards {
        /// Topic or focus area
        topic: Option<String>,
    },
    /// Generate a practice quiz
    Quiz {
        /// Topic or focus area
        topic: Option<String>,
    },
    /// Generate a summary of materials
    Summary {
        /// Topic or document to summarize
        topic: Option<String>,
    },
    /// Interactive homework help mode
    Homework,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add { path }) => {
            commands::bucket::print_bucket_context();
            commands::add::run(path).await?;
        }
        Some(Commands::Chat) => {
            commands::bucket::print_bucket_context();
            commands::chat::run().await?;
        }
        Some(Commands::List) => {
            commands::bucket::print_bucket_context();
            commands::docs::list().await?;
        }
        Some(Commands::Search { query }) => {
            commands::bucket::print_bucket_context();
            commands::docs::search(query).await?;
        }
        Some(Commands::Docs) => {
            commands::bucket::print_bucket_context();
            commands::docs::run().await?;
        }
        Some(Commands::Delete { id }) => {
            commands::bucket::print_bucket_context();
            commands::docs::delete(id).await?;
        }
        Some(Commands::Bucket { action }) => match action {
            Some(BucketAction::Create { name }) => {
                commands::bucket::create(name).await?;
            }
            Some(BucketAction::List) => {
                commands::bucket::list().await?;
            }
            Some(BucketAction::Use { name }) => {
                commands::bucket::switch(name).await?;
            }
            Some(BucketAction::Delete { name: _ }) => {
                // Interactive delete
                commands::bucket::run().await?;
            }
            None => {
                commands::bucket::run().await?;
            }
        },
        Some(Commands::Config) => {
            commands::config::run().await?;
        }
        Some(Commands::Generate { action }) => {
            commands::bucket::print_bucket_context();
            match action {
                Some(GenerateAction::StudyGuide { topic }) => {
                    commands::generate::study_guide(topic).await?;
                }
                Some(GenerateAction::Flashcards { topic }) => {
                    commands::generate::flashcards(topic).await?;
                }
                Some(GenerateAction::Quiz { topic }) => {
                    commands::generate::quiz(topic).await?;
                }
                Some(GenerateAction::Summary { topic }) => {
                    commands::generate::summary(topic).await?;
                }
                Some(GenerateAction::Homework) => {
                    commands::generate::homework_help().await?;
                }
                None => {
                    commands::generate::run().await?;
                }
            }
        }
        Some(Commands::Review) => {
            commands::bucket::print_bucket_context();
            commands::review::run().await?;
        }
        Some(Commands::Quiz) => {
            commands::bucket::print_bucket_context();
            commands::quiz::run().await?;
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
        }
        None => {
            // No subcommand - launch the full-screen TUI
            tui::run().await?;
        }
    }

    Ok(())
}

/// Display the library shelf with buckets as books
#[allow(dead_code)]
fn print_library_shelf() {
    let buckets = bucket::Bucket::list_all().unwrap_or_default();
    let current = bucket::get_current_bucket().ok().flatten().map(|b| b.name);

    if buckets.is_empty() {
        println!(
            "    {}",
            "┌─────────────────────────────────────────────┐".dimmed()
        );
        println!(
            "    {}  {}  {}",
            "│".dimmed(),
            "Your library is empty. Add a bucket!".yellow(),
            "│".dimmed()
        );
        println!(
            "    {}",
            "└─────────────────────────────────────────────┘".dimmed()
        );
        println!(
            "    {}",
            "═══════════════════════════════════════════════".dimmed()
        );
        return;
    }

    // Draw shelf with books
    println!(
        "    {}",
        "┌─────────────────────────────────────────────┐".cyan()
    );
    println!(
        "    {}       {}       {}",
        "│".cyan(),
        "📚 YOUR LIBRARY 📚".bold().white(),
        "│".cyan()
    );
    println!(
        "    {}",
        "├─────────────────────────────────────────────┤".cyan()
    );

    // Draw books on shelf
    let mut book_row = String::from("    │ ");
    for bucket_name in &buckets {
        let is_current = current.as_ref() == Some(bucket_name);
        let book = if is_current {
            format!(" 📖 {} ", bucket_name)
                .on_cyan()
                .black()
                .to_string()
        } else {
            format!(" 📕 {} ", bucket_name).to_string()
        };
        book_row.push_str(&book);
        book_row.push_str("  ");
    }
    // Pad to fit box
    let display_len = book_row.chars().count();
    if display_len < 50 {
        book_row.push_str(&" ".repeat(50 - display_len));
    }
    book_row.push('│');
    println!("{}", book_row.cyan());

    println!(
        "    {}",
        "└─────────────────────────────────────────────┘".cyan()
    );
    println!(
        "    {}",
        "▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀".yellow()
    );
}

/// Print the status dashboard
#[allow(dead_code)]
fn print_dashboard() {
    // Get bucket info
    let current_bucket = bucket::get_current_bucket().ok().flatten();
    let bucket_name = current_bucket
        .as_ref()
        .map(|b| b.name.clone())
        .unwrap_or_else(|| "(no bucket selected)".to_string());

    // Get document count
    let doc_count = storage::Database::open()
        .and_then(|db| {
            let store = storage::DocumentStore::new(&db);
            store.count()
        })
        .unwrap_or(0);

    // Get chunk count
    let chunk_count = storage::Database::open()
        .and_then(|db| {
            let store = storage::ChunkStore::new(&db);
            store.count()
        })
        .unwrap_or(0);

    // Check API key status
    let has_api_key = config::Config::load()
        .map(|c| c.has_api_key())
        .unwrap_or(false);

    println!();
    println!(
        "    {}",
        "╭──────────────────── STATUS ────────────────────╮".bright_black()
    );
    println!(
        "    {}  {} {}",
        "│".bright_black(),
        "📖 Current Book:".bold(),
        if current_bucket.is_some() {
            bucket_name.cyan().to_string()
        } else {
            bucket_name.dimmed().to_string()
        }
    );
    println!(
        "    {}  {} {} documents, {} chunks",
        "│".bright_black(),
        "📄 Contents:".bold(),
        doc_count.to_string().green(),
        chunk_count.to_string().green()
    );
    println!(
        "    {}  {} {}",
        "│".bright_black(),
        "🔑 API Key:".bold(),
        if has_api_key {
            "Ready".green().to_string()
        } else {
            "Not configured".red().to_string()
        }
    );
    println!(
        "    {}",
        "╰─────────────────────────────────────────────────╯".bright_black()
    );
    println!();
}

#[allow(dead_code)]
async fn run_interactive() -> Result<()> {
    use inquire::Select;

    // Print the banner once at start
    print_banner();

    // Show version info
    println!(
        "    {} {} │ {} {}",
        "Version".dimmed(),
        env!("CARGO_PKG_VERSION").cyan(),
        "Powered by".dimmed(),
        "Groq + FastEmbed".green()
    );

    // Main application loop
    loop {
        // Show library shelf
        println!();
        print_library_shelf();

        // Show status dashboard
        print_dashboard();

        let options = vec![
            "📥  Add Knowledge        │ Import files, URLs, videos",
            "💬  Ask the Librarian    │ Chat with your materials",
            "📝  Study Tools          │ Generate guides, flashcards, quizzes",
            "🔁  Review               │ Spaced repetition study session",
            "🎯  Quiz                 │ Test your knowledge interactively",
            "───────────────────────────────────────────────",
            "📋  Browse Collection    │ List all documents",
            "🔍  Search               │ Find specific content",
            "📂  Manage Documents     │ View, edit, delete",
            "📚  Manage Library       │ Create, switch, delete buckets",
            "───────────────────────────────────────────────",
            "⚙️   Settings            │ API keys, preferences",
            "🚪  Exit                 │ Close The Librarian",
        ];

        let selection = Select::new("What would you like to do?", options)
            .with_help_message("↑↓ navigate • Enter select • Esc back")
            .prompt();

        // Handle Escape/Ctrl+C gracefully
        let selection = match selection {
            Ok(s) => s,
            Err(inquire::InquireError::OperationCanceled) => {
                print_farewell();
                break;
            }
            Err(inquire::InquireError::OperationInterrupted) => {
                print_farewell();
                break;
            }
            Err(e) => return Err(e.into()),
        };

        // Skip separator lines
        if selection.starts_with("───") {
            continue;
        }

        println!(); // Add spacing

        // Execute the selected action, catching errors gracefully
        let result = match selection {
            s if s.contains("Add Knowledge") => commands::add::run(None).await,
            s if s.contains("Ask the Librarian") => commands::chat::run().await,
            s if s.contains("Study Tools") => commands::generate::run().await,
            s if s.contains("Review") => commands::review::run().await,
            s if s.contains("Quiz") => commands::quiz::run().await,
            s if s.contains("Browse Collection") => commands::docs::list().await,
            s if s.contains("Search") => commands::docs::search(None).await,
            s if s.contains("Manage Documents") => commands::docs::run().await,
            s if s.contains("Manage Library") => commands::bucket::run().await,
            s if s.contains("Settings") => commands::config::run().await,
            s if s.contains("Exit") => {
                print_farewell();
                break;
            }
            _ => continue,
        };

        // Handle errors from commands gracefully - show error but continue
        if let Err(e) = result {
            let err_str = e.to_string();
            if err_str.contains("cancelled") || err_str.contains("interrupted") {
                println!("\n    {}", "← Returning to main menu...".dimmed());
            } else {
                eprintln!("\n    {} {}", "Error:".red(), e);
            }
        }

        println!(); // Add spacing before next iteration
    }

    Ok(())
}

#[allow(dead_code)]
fn print_farewell() {
    println!();
    println!(
        "    {}",
        "╭─────────────────────────────────────────╮".cyan()
    );
    println!(
        "    {}   {}   {}",
        "│".cyan(),
        "📚 Thanks for visiting The Librarian! 📚".bold(),
        "│".cyan()
    );
    println!(
        "    {}          {}          {}",
        "│".cyan(),
        "Happy studying! 🎓".green(),
        "│".cyan()
    );
    println!(
        "    {}",
        "╰─────────────────────────────────────────╯".cyan()
    );
    println!();
}
