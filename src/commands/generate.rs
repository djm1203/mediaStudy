use anyhow::Result;
use colored::Colorize;
use inquire::{Select, Text};
use std::io::Write;

use crate::bucket;
use crate::config::Config;
use crate::llm::GroqClient;
use crate::storage::{ChunkStore, Database, DocumentStore};

/// Prompts for different generation types
mod prompts {
    pub const STUDY_GUIDE: &str = r#"You are creating a comprehensive study guide from the provided course materials.

Create a well-organized study guide that includes:
1. **Key Concepts** - Main ideas and definitions
2. **Important Details** - Supporting facts and examples
3. **Relationships** - How concepts connect to each other
4. **Summary Points** - Quick review bullets

Format the output in clean Markdown. Be thorough but concise.
Include section headers and use bullet points for easy scanning.
Cite specific documents when referencing information: [Source: filename]"#;

    pub const FLASHCARDS: &str = r#"You are creating flashcards for studying from the provided course materials.

Generate flashcards in this exact format:
---
Q: [Question]
A: [Answer]
---

Rules:
- Create 10-15 flashcards covering key concepts
- Questions should test understanding, not just recall
- Answers should be concise but complete
- Cover the most important material first
- Include a mix of definition, concept, and application questions"#;

    pub const QUIZ: &str = r#"You are creating a practice quiz from the provided course materials.

Generate a quiz with mixed question types:

## Multiple Choice
1. Question text
   a) Option A
   b) Option B
   c) Option C
   d) Option D
   **Answer: b)**

## Fill in the Blank
1. The process of _______ is essential for...
   **Answer: [correct answer]**

## Short Answer
1. Explain the concept of...
   **Answer: [brief expected answer]**

Rules:
- Create 10 questions total (mix of types)
- Base questions only on the provided materials
- Include answers after each question
- Progress from easier to harder questions"#;

    pub const SUMMARY: &str = r#"You are creating a concise summary of the provided course materials.

Create a summary that:
1. Captures the main thesis/topic
2. Lists key points in order of importance
3. Highlights critical terms and definitions
4. Notes any formulas, processes, or frameworks
5. Ends with 3-5 takeaway points

Keep the summary focused and scannable. Use bullet points and headers.
Target length: 300-500 words."#;

    pub const HOMEWORK_HELP: &str = r#"You are a tutor helping a student with their homework using their course materials.

Guidelines:
1. Guide the student toward understanding - don't just give answers
2. Reference specific concepts from their materials
3. Break down complex problems into steps
4. Ask clarifying questions if the problem is unclear
5. Provide examples similar to what's in their materials

If the problem requires knowledge not in the materials, note what additional concepts might be needed."#;
}

pub async fn run() -> Result<()> {
    println!("\n{}", "╔══════════════════════════════════════════════════╗".magenta());
    println!("{}", "║         📝  STUDY MATERIAL GENERATOR             ║".magenta());
    println!("{}", "╚══════════════════════════════════════════════════╝".magenta());
    println!();

    let options = vec![
        "Study Guide",
        "Flashcards",
        "Quiz",
        "Summary",
        "Homework Help",
        "Back",
    ];

    let selection = Select::new("What would you like to generate?", options).prompt()?;

    match selection {
        "Study Guide" => study_guide(None).await?,
        "Flashcards" => flashcards(None).await?,
        "Quiz" => quiz(None).await?,
        "Summary" => summary(None).await?,
        "Homework Help" => homework_help().await?,
        "Back" => {}
        _ => unreachable!(),
    }

    Ok(())
}

/// Generate a study guide
pub async fn study_guide(topic: Option<String>) -> Result<()> {
    let topic = match topic {
        Some(t) => t,
        None => Text::new("Topic or focus area (or press Enter for all materials):")
            .prompt()
            .unwrap_or_default(),
    };

    generate_content("Study Guide", prompts::STUDY_GUIDE, &topic).await
}

/// Generate flashcards
pub async fn flashcards(topic: Option<String>) -> Result<()> {
    let topic = match topic {
        Some(t) => t,
        None => Text::new("Topic or focus area (or press Enter for all materials):")
            .prompt()
            .unwrap_or_default(),
    };

    generate_content("Flashcards", prompts::FLASHCARDS, &topic).await
}

/// Generate a quiz
pub async fn quiz(topic: Option<String>) -> Result<()> {
    let topic = match topic {
        Some(t) => t,
        None => Text::new("Topic or focus area (or press Enter for all materials):")
            .prompt()
            .unwrap_or_default(),
    };

    generate_content("Quiz", prompts::QUIZ, &topic).await
}

/// Generate a summary
pub async fn summary(topic: Option<String>) -> Result<()> {
    let topic = match topic {
        Some(t) => t,
        None => Text::new("Topic or document to summarize (or press Enter for all):")
            .prompt()
            .unwrap_or_default(),
    };

    generate_content("Summary", prompts::SUMMARY, &topic).await
}

/// Interactive homework help
pub async fn homework_help() -> Result<()> {
    let config = Config::load()?;
    let api_key = match config.get_api_key() {
        Some(key) => key,
        None => {
            println!(
                "{} No API key configured. Run {} to set up.",
                "Error:".red().bold(),
                "media-study config".cyan()
            );
            return Ok(());
        }
    };

    let client = GroqClient::new(api_key, config.default_model);

    // Get context
    let context = get_document_context("")?;

    if context.is_empty() {
        println!(
            "{} No documents in current bucket. Add materials first.",
            "Error:".red()
        );
        return Ok(());
    }

    println!("{}", "Homework Help Mode".bold().cyan());
    println!("{}", "─".repeat(40).dimmed());
    println!("Type your homework question or problem.");
    println!("Type {} to exit.\n", "done".dimmed());

    let mut conversation = vec![
        crate::llm::groq::Message {
            role: "system".to_string(),
            content: prompts::HOMEWORK_HELP.to_string(),
        },
    ];

    loop {
        let input = Text::new("Problem:")
            .with_help_message("Describe your homework problem")
            .prompt()?;

        let input = input.trim();

        if input.eq_ignore_ascii_case("done") || input.eq_ignore_ascii_case("exit") {
            println!("{}", "Good luck with your studies!".dimmed());
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Add context and question
        let user_message = format!(
            "COURSE MATERIALS:\n{}\n\n---\n\nHOMEWORK PROBLEM: {}",
            context, input
        );

        conversation.push(crate::llm::groq::Message {
            role: "user".to_string(),
            content: user_message,
        });

        print!("{} ", "Tutor:".magenta().bold());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match client.chat_stream(&conversation).await {
            Ok(response) => {
                println!(); // Extra newline after streaming

                // Store simplified version for history
                if let Some(last_msg) = conversation.last_mut() {
                    last_msg.content = input.to_string();
                }
                conversation.push(crate::llm::groq::Message {
                    role: "assistant".to_string(),
                    content: response,
                });
            }
            Err(e) => {
                println!("\n{} {}\n", "Error:".red(), e);
                conversation.pop();
            }
        }
    }

    Ok(())
}

/// Core generation function
async fn generate_content(name: &str, system_prompt: &str, topic: &str) -> Result<()> {
    let config = Config::load()?;

    let api_key = match config.get_api_key() {
        Some(key) => key,
        None => {
            println!(
                "{} No API key configured. Run {} to set up.",
                "Error:".red().bold(),
                "media-study config".cyan()
            );
            return Ok(());
        }
    };

    let client = GroqClient::new(api_key, config.default_model);

    // Get document context
    let context = get_document_context(topic)?;

    if context.is_empty() {
        println!(
            "{} No documents found in current bucket. Add materials first with {}",
            "Error:".red(),
            "media-study add".cyan()
        );
        return Ok(());
    }

    let bucket_name = bucket::get_current_bucket()?
        .map(|b| b.name)
        .unwrap_or_else(|| "(default)".to_string());

    println!("\n{} {}", "Bucket:".dimmed(), bucket_name.cyan());
    println!(
        "{} {}",
        "Generating:".dimmed(),
        name.yellow()
    );
    if !topic.is_empty() {
        println!("{} {}", "Focus:".dimmed(), topic);
    }
    print!("{} ", "Working...".dimmed());

    // Build the request
    let user_message = if topic.is_empty() {
        format!(
            "Create a {} from the following course materials:\n\n{}",
            name.to_lowercase(),
            context
        )
    } else {
        format!(
            "Create a {} focused on '{}' from the following course materials:\n\n{}",
            name.to_lowercase(),
            topic,
            context
        )
    };

    let messages = vec![
        crate::llm::groq::Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        crate::llm::groq::Message {
            role: "user".to_string(),
            content: user_message,
        },
    ];

    // Clear the "Working..." message and start streaming
    print!("\r{}\r", " ".repeat(20));
    println!("\n{}", "─".repeat(50).dimmed());
    std::io::stdout().flush().ok();

    match client.chat_stream(&messages).await {
        Ok(response) => {
            println!("{}", "─".repeat(50).dimmed());

            // Offer to save
            let save = Select::new("Save to file?", vec!["No", "Yes"]).prompt()?;

            if save == "Yes" {
                let default_name = format!(
                    "{}-{}.md",
                    name.to_lowercase().replace(' ', "-"),
                    chrono::Local::now().format("%Y%m%d-%H%M")
                );

                let filename = Text::new("Filename:")
                    .with_default(&default_name)
                    .prompt()?;

                std::fs::write(&filename, &response)?;
                println!("{} Saved to {}", "✓".green(), filename.cyan());
            }
        }
        Err(e) => {
            println!("{} {}", "Error:".red(), e);
        }
    }

    Ok(())
}

/// Get document context for generation
fn get_document_context(topic: &str) -> Result<String> {
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);

    // Initialize chunks table
    chunk_store.init_schema()?;

    let chunk_count = chunk_store.count().unwrap_or(0);

    // If we have chunks and a topic, use semantic search
    if chunk_count > 0 && !topic.is_empty() {
        if let Ok(context) = build_semantic_context(&chunk_store, &doc_store, topic) {
            if !context.is_empty() {
                return Ok(context);
            }
        }
    }

    // Otherwise, use all documents (up to a limit)
    let documents = if topic.is_empty() {
        doc_store.list()?
    } else {
        let results = doc_store.search(topic)?;
        if results.is_empty() {
            doc_store.list()?
        } else {
            results
        }
    };

    if documents.is_empty() {
        return Ok(String::new());
    }

    let mut context = String::new();
    let mut total_chars = 0;
    const MAX_CONTEXT_CHARS: usize = 10000; // More context for generation

    for doc in documents.iter().take(10) {
        if total_chars >= MAX_CONTEXT_CHARS {
            break;
        }

        let remaining = MAX_CONTEXT_CHARS - total_chars;
        let content = if doc.content.len() > remaining {
            &doc.content[..remaining]
        } else {
            &doc.content
        };

        context.push_str(&format!(
            "--- Document: {} ---\n{}\n\n",
            doc.filename, content
        ));

        total_chars += content.len() + doc.filename.len() + 30;
    }

    Ok(context)
}

/// Build semantic context using embeddings
fn build_semantic_context(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    query: &str,
) -> Result<String> {
    use crate::embeddings;

    let query_embedding = embeddings::embed_text(query)?;
    let chunks = chunk_store.get_all_with_embeddings()?;

    if chunks.is_empty() {
        return Ok(String::new());
    }

    let chunk_embeddings: Vec<(i64, Vec<f32>)> = chunks
        .iter()
        .filter_map(|c| c.embedding.as_ref().map(|e| (c.id, e.clone())))
        .collect();

    let similar = embeddings::find_similar(&query_embedding, &chunk_embeddings, 10);

    let mut context = String::new();
    let mut total_chars = 0;
    const MAX_CONTEXT_CHARS: usize = 10000;

    let similar_ids: Vec<i64> = similar.iter().map(|(id, _)| *id).collect();

    for chunk in &chunks {
        if !similar_ids.contains(&chunk.id) {
            continue;
        }

        if total_chars >= MAX_CONTEXT_CHARS {
            break;
        }

        let doc = doc_store.get(chunk.document_id)?;
        let filename = doc.map(|d| d.filename).unwrap_or_else(|| "Unknown".to_string());

        context.push_str(&format!(
            "--- {} ---\n{}\n\n",
            filename, chunk.content
        ));

        total_chars += chunk.content.len() + filename.len() + 20;
    }

    Ok(context)
}
