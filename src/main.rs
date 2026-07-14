use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
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

use tui::Screen;

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
        /// Path or URL to add (opens the TUI Add screen if omitted)
        path: Option<String>,
    },
    /// Ask the Librarian - chat with your materials
    Chat,
    /// Browse your collection
    List,
    /// Search your materials
    Search {
        /// Search query (opens the TUI Search screen if omitted)
        query: Option<String>,
    },
    /// Manage documents
    Docs,
    /// Remove a document from your collection
    Delete {
        /// Document ID to delete (opens the TUI Docs screen if omitted)
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // `add <path>` is headless; a bare `add` opens the TUI Add screen.
        Some(Commands::Add { path }) => match path {
            Some(p) => {
                commands::bucket::print_bucket_context();
                commands::add::run(Some(p)).await?;
            }
            None => tui::run_on(Screen::Add).await?,
        },
        Some(Commands::Chat) => tui::run_on(Screen::Chat).await?,
        Some(Commands::List) => {
            commands::bucket::print_bucket_context();
            commands::docs::list().await?;
        }
        // `search <query>` is headless; a bare `search` opens the TUI Search screen.
        Some(Commands::Search { query }) => match query {
            Some(q) => {
                commands::bucket::print_bucket_context();
                commands::docs::search(Some(q)).await?;
            }
            None => tui::run_on(Screen::Search).await?,
        },
        Some(Commands::Docs) => tui::run_on(Screen::Docs).await?,
        // `delete <id>` is headless; a bare `delete` opens the TUI Docs screen.
        Some(Commands::Delete { id }) => match id {
            Some(id) => {
                commands::bucket::print_bucket_context();
                commands::docs::delete(Some(id)).await?;
            }
            None => tui::run_on(Screen::Docs).await?,
        },
        Some(Commands::Bucket { action }) => match action {
            None => tui::run_on(Screen::Home).await?,
            Some(BucketAction::Create { name }) => match name {
                Some(_) => commands::bucket::create(name).await?,
                None => tui::run_on(Screen::Home).await?,
            },
            Some(BucketAction::List) => commands::bucket::list().await?,
            Some(BucketAction::Use { name }) => match name {
                Some(_) => commands::bucket::switch(name).await?,
                None => tui::run_on(Screen::Home).await?,
            },
            Some(BucketAction::Delete { name }) => match name {
                Some(_) => commands::bucket::delete(name).await?,
                None => tui::run_on(Screen::Home).await?,
            },
        },
        Some(Commands::Config) => tui::run_on(Screen::Config).await?,
        // `generate <kind> [topic]` is headless; a bare `generate` opens the TUI Study screen.
        Some(Commands::Generate { action }) => match action {
            None => tui::run_on(Screen::Study).await?,
            Some(act) => {
                commands::bucket::print_bucket_context();
                match act {
                    GenerateAction::StudyGuide { topic } => {
                        commands::generate::study_guide(topic).await?
                    }
                    GenerateAction::Flashcards { topic } => {
                        commands::generate::flashcards(topic).await?
                    }
                    GenerateAction::Quiz { topic } => commands::generate::quiz(topic).await?,
                    GenerateAction::Summary { topic } => commands::generate::summary(topic).await?,
                }
            }
        },
        Some(Commands::Review) => tui::run_on(Screen::Review).await?,
        Some(Commands::Quiz) => tui::run_on(Screen::Quiz).await?,
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
        }
        // No subcommand - launch the full-screen TUI on Home.
        None => tui::run().await?,
    }

    Ok(())
}
