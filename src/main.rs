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
mod storage;

/// ASCII art banner for the application
const BANNER: &str = r#"
  __  __          _ _       ____  _             _
 |  \/  | ___  __| (_) __ _/ ___|| |_ _   _  __| |_   _
 | |\/| |/ _ \/ _` | |/ _` \___ \| __| | | |/ _` | | | |
 | |  | |  __/ (_| | | (_| |___) | |_| |_| | (_| | |_| |
 |_|  |_|\___|\__,_|_|\__,_|____/ \__|\__,_|\__,_|\__, |
                                                  |___/
"#;

/// Print the application banner
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

/// Print a styled status line
fn print_status(label: &str, value: &str, icon: &str) {
    println!(
        "  {} {} {}",
        icon,
        format!("{}:", label).dimmed(),
        value.cyan()
    );
}

#[derive(Parser)]
#[command(name = "media-study")]
#[command(about = "CLI tool for ingesting media and studying with LLM assistance")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add content (files, directories, URLs)
    Add {
        /// Path or URL to add (skips interactive prompt if provided)
        path: Option<String>,
    },
    /// Start an interactive chat session
    Chat,
    /// List all ingested documents
    List,
    /// Search documents by content
    Search {
        /// Search query
        query: Option<String>,
    },
    /// Manage documents (view, delete)
    Docs,
    /// Delete a document by ID
    Delete {
        /// Document ID to delete
        id: Option<i64>,
    },
    /// Manage knowledge buckets (datasets per class/project)
    Bucket {
        #[command(subcommand)]
        action: Option<BucketAction>,
    },
    /// Configure settings (API keys, preferences)
    Config,
    /// Generate study materials (guides, flashcards, quizzes)
    Generate {
        #[command(subcommand)]
        action: Option<GenerateAction>,
    },
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
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
        }
        None => {
            // No subcommand - show interactive menu
            run_interactive().await?;
        }
    }

    Ok(())
}

async fn run_interactive() -> Result<()> {
    use inquire::Select;

    // Print the cool banner
    print_banner();

    // Show version and description
    println!(
        "  {} {}",
        "Version:".dimmed(),
        env!("CARGO_PKG_VERSION").cyan()
    );
    println!(
        "  {} {}\n",
        "Powered by:".dimmed(),
        "Groq LLM + Local Embeddings".green()
    );

    // Show current status
    println!("{}", "─".repeat(50).dimmed());

    // Get bucket info
    let bucket_name = bucket::get_current_bucket()
        .ok()
        .flatten()
        .map(|b| b.name)
        .unwrap_or_else(|| "(default)".to_string());

    // Get document count
    let doc_count = storage::Database::open()
        .and_then(|db| {
            let store = storage::DocumentStore::new(&db);
            store.count()
        })
        .unwrap_or(0);

    print_status("Bucket", &bucket_name, "📚");
    print_status("Documents", &doc_count.to_string(), "📄");

    // Check API key status
    let has_api_key = config::Config::load()
        .map(|c| c.has_api_key())
        .unwrap_or(false);

    let api_status = if has_api_key {
        "Configured".green().to_string()
    } else {
        "Not set (run 'config')".red().to_string()
    };
    print_status("API Key", &api_status, "🔑");

    println!("{}\n", "─".repeat(50).dimmed());

    let options = vec![
        "📥  Add content (files, URLs, videos)",
        "💬  Chat with your materials",
        "📝  Generate study materials",
        "📋  List documents",
        "🔍  Search documents",
        "📂  Manage documents",
        "🗂️   Manage buckets",
        "⚙️   Configure settings",
        "🚪  Exit",
    ];

    let selection = Select::new("What would you like to do?", options)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()?;

    println!(); // Add spacing

    match selection {
        s if s.contains("Add content") => commands::add::run(None).await?,
        s if s.contains("Chat with") => commands::chat::run().await?,
        s if s.contains("Generate study") => commands::generate::run().await?,
        s if s.contains("List documents") => commands::docs::list().await?,
        s if s.contains("Search documents") => commands::docs::search(None).await?,
        s if s.contains("Manage documents") => commands::docs::run().await?,
        s if s.contains("Manage buckets") => commands::bucket::run().await?,
        s if s.contains("Configure") => commands::config::run().await?,
        s if s.contains("Exit") => {
            println!(
                "{}",
                "👋 Thanks for using Media Study! Happy learning!".cyan()
            );
        }
        _ => unreachable!(),
    }

    Ok(())
}
