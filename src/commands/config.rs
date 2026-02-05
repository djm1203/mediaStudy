use anyhow::Result;
use colored::Colorize;
use inquire::{Password, Select};

use crate::config::Config;
use crate::llm::GroqClient;

pub async fn run() -> Result<()> {
    println!();
    println!(
        "    {}",
        "╭──────────────────────────────────────────────────────╮".bright_black()
    );
    println!(
        "    {}            {}            {}",
        "│".bright_black(),
        "⚙️  SETTINGS ⚙️".bold().white(),
        "│".bright_black()
    );
    println!(
        "    {}        {}        {}",
        "│".bright_black(),
        "Configure The Librarian to your liking".dimmed(),
        "│".bright_black()
    );
    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".bright_black()
    );
    println!();

    let mut config = Config::load()?;

    let options = vec![
        "🔑  Set API Key        │ Configure Groq API access",
        "🤖  Select Model       │ Choose default LLM",
        "📋  View Settings      │ See current configuration",
        "←   Back",
    ];

    loop {
        let selection =
            Select::new("What would you like to configure?", options.clone()).prompt();

        let selection = match selection {
            Ok(s) => s,
            Err(inquire::InquireError::OperationCanceled)
            | Err(inquire::InquireError::OperationInterrupted) => break,
            Err(e) => return Err(e.into()),
        };

        match selection {
            s if s.contains("Set API Key") => {
                if let Err(e) = set_api_key(&mut config).await {
                    if !e.to_string().contains("cancelled") {
                        eprintln!("{} {}", "Error:".red(), e);
                    }
                }
            }
            s if s.contains("Select Model") => {
                if let Err(e) = select_model(&mut config).await {
                    if !e.to_string().contains("cancelled") {
                        eprintln!("{} {}", "Error:".red(), e);
                    }
                }
            }
            s if s.contains("View Settings") => {
                view_config(&config);
            }
            s if s.contains("Back") => break,
            _ => {}
        }

        println!();
    }

    Ok(())
}

async fn set_api_key(config: &mut Config) -> Result<()> {
    println!(
        "\n{} Get your API key from {}",
        "Tip:".yellow(),
        "https://console.groq.com/keys".cyan()
    );

    let key = Password::new("Enter your Groq API key:")
        .without_confirmation()
        .prompt()?;

    if key.is_empty() {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    }

    config.groq_api_key = Some(key);
    config.save()?;

    println!("{} API key saved!", "✓".green());

    Ok(())
}

async fn select_model(config: &mut Config) -> Result<()> {
    let model_options: Vec<String> = GroqClient::MODELS
        .iter()
        .map(|(id, desc)| format!("{} - {}", id, desc))
        .collect();

    let selection = Select::new("Select default model:", model_options).prompt()?;

    // Extract model ID from selection
    let model_id = selection.split(" - ").next().unwrap().to_string();

    config.default_model = Some(model_id.clone());
    config.save()?;

    println!("{} Default model set to {}", "✓".green(), model_id.yellow());

    Ok(())
}

fn view_config(config: &Config) {
    println!("\n{}", "Current Configuration:".bold());
    println!("{}", "─".repeat(30).dimmed());

    let api_status = if config.has_api_key() {
        "configured".green().to_string()
    } else if std::env::var("GROQ_API_KEY").is_ok() {
        "set via GROQ_API_KEY env".yellow().to_string()
    } else {
        "not set".red().to_string()
    };

    println!("  API Key: {}", api_status);

    println!(
        "  Default Model: {}",
        config
            .default_model
            .as_deref()
            .unwrap_or("llama-3.3-70b-versatile (default)")
    );

    if let Ok(path) = Config::config_path() {
        println!("  Config file: {}", path.display().to_string().dimmed());
    }

    if let Ok(path) = Config::data_dir() {
        println!("  Data directory: {}", path.display().to_string().dimmed());
    }
}
