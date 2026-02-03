use anyhow::Result;
use colored::Colorize;
use inquire::Text;

use crate::bucket;
use crate::config::Config;
use crate::embeddings;
use crate::llm::{GroqClient, groq::Message};
use crate::storage::{ChunkStore, Database, DocumentStore};

const GROUNDED_SYSTEM_PROMPT: &str = r#"You are a study assistant helping a student learn from their course materials.

IMPORTANT INSTRUCTIONS:
1. Answer questions using ONLY the provided context from their documents
2. If the answer is not in the provided context, say "I don't have information about this in your materials"
3. When you use information from the context, cite which document it came from
4. Be concise but thorough in your explanations
5. If asked to explain a concept, use examples from the provided materials when possible

Format citations like: [Source: filename]"#;

const NO_DOCS_SYSTEM_PROMPT: &str = r#"You are a study assistant. The user has no documents loaded in their current bucket.

Help them by:
1. Answering general questions to the best of your ability
2. Suggesting they add study materials with 'media-study add <file>'
3. Being clear when you're using general knowledge vs. their specific materials"#;

pub async fn run() -> Result<()> {
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

    // Check current bucket and document count
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);

    // Initialize chunks table if needed
    chunk_store.init_schema()?;

    let doc_count = doc_store.count()?;
    let chunk_count = chunk_store.count().unwrap_or(0);

    let bucket_name = bucket::get_current_bucket()?
        .map(|b| b.name)
        .unwrap_or_else(|| "(default)".to_string());

    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║            💬  INTERACTIVE CHAT                  ║".cyan()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════╝".cyan()
    );
    println!();
    println!("  {} {}", "📚 Bucket:".dimmed(), bucket_name.cyan().bold());
    println!(
        "  {} {}",
        "📄 Documents:".dimmed(),
        doc_count.to_string().yellow()
    );
    println!(
        "  {} {}",
        "🧩 Chunks:".dimmed(),
        chunk_count.to_string().white()
    );
    println!("  {} {}", "🤖 Model:".dimmed(), client.model.green());
    println!();
    println!(
        "{}",
        "──────────────────────────────────────────────────".dimmed()
    );
    println!("  Type {} to exit", "quit".yellow().bold());
    println!(
        "{}\n",
        "──────────────────────────────────────────────────".dimmed()
    );

    if doc_count == 0 {
        println!(
            "{} No documents in this bucket. Add some with {}",
            "Note:".yellow(),
            "media-study add <file>".cyan()
        );
        println!("Chat will use general knowledge only.\n");
    } else if chunk_count == 0 {
        println!(
            "{} Documents exist but no chunks/embeddings. Re-add documents to enable semantic search.\n",
            "Note:".yellow()
        );
    }

    // Choose system prompt based on whether we have documents
    let system_prompt = if doc_count > 0 {
        GROUNDED_SYSTEM_PROMPT
    } else {
        NO_DOCS_SYSTEM_PROMPT
    };

    let mut conversation: Vec<Message> = vec![Message {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    }];

    loop {
        let input = Text::new("You:")
            .with_help_message("Ask a question or type 'quit' to exit")
            .prompt()?;

        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("{}", "Goodbye!".dimmed());
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Search for relevant context using semantic search
        let context = if chunk_count > 0 {
            build_semantic_context(&chunk_store, &doc_store, input)?
        } else if doc_count > 0 {
            // Fallback to FTS if no chunks
            build_fts_context(&doc_store, input)?
        } else {
            String::new()
        };

        // Build the user message with context
        let user_message = if context.is_empty() {
            input.to_string()
        } else {
            format!(
                "CONTEXT FROM YOUR STUDY MATERIALS:\n{}\n\n---\n\nQUESTION: {}",
                context, input
            )
        };

        conversation.push(Message {
            role: "user".to_string(),
            content: user_message,
        });

        // Show status briefly then clear for streaming output
        print!("{}", "Searching context...".dimmed());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        // Small delay to show the searching message
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        print!("\r{}\r", " ".repeat(25));

        print!("{} ", "Assistant:".green().bold());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match client.chat_stream(&conversation).await {
            Ok(response) => {
                println!(); // Extra newline after streaming

                // Store just the question (not the context) for conversation history
                if let Some(last_msg) = conversation.last_mut() {
                    last_msg.content = input.to_string();
                }
                conversation.push(Message {
                    role: "assistant".to_string(),
                    content: response,
                });
            }
            Err(e) => {
                println!("\n{} {}\n", "Error:".red().bold(), e);
                conversation.pop();
            }
        }
    }

    Ok(())
}

/// Build context using semantic search (embeddings)
fn build_semantic_context(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    query: &str,
) -> Result<String> {
    // Generate query embedding
    let query_embedding = match embeddings::embed_text(query) {
        Ok(emb) => emb,
        Err(_) => return build_fts_context(doc_store, query), // Fallback to FTS
    };

    // Get all chunks with embeddings
    let chunks = chunk_store.get_all_with_embeddings()?;

    if chunks.is_empty() {
        return build_fts_context(doc_store, query);
    }

    // Convert to format for similarity search
    let chunk_embeddings: Vec<(i64, Vec<f32>)> = chunks
        .iter()
        .filter_map(|c| c.embedding.as_ref().map(|e| (c.id, e.clone())))
        .collect();

    // Find top 5 most similar chunks
    let similar = embeddings::find_similar(&query_embedding, &chunk_embeddings, 5);

    if similar.is_empty() {
        return build_fts_context(doc_store, query);
    }

    // Build context from similar chunks
    let mut context = String::new();
    let mut total_chars = 0;
    const MAX_CONTEXT_CHARS: usize = 6000;

    // Get chunk IDs and their similarities
    let similar_ids: Vec<i64> = similar.iter().map(|(id, _)| *id).collect();

    // Find the chunks and their documents
    for chunk in &chunks {
        if !similar_ids.contains(&chunk.id) {
            continue;
        }

        if total_chars >= MAX_CONTEXT_CHARS {
            break;
        }

        // Get document filename
        let doc = doc_store.get(chunk.document_id)?;
        let filename = doc
            .map(|d| d.filename)
            .unwrap_or_else(|| "Unknown".to_string());

        let remaining = MAX_CONTEXT_CHARS - total_chars;
        let content = truncate_content(&chunk.content, remaining.min(1500));

        context.push_str(&format!(
            "--- Document: {} (chunk {}) ---\n{}\n\n",
            filename, chunk.chunk_index, content
        ));

        total_chars += content.len() + filename.len() + 50;
    }

    Ok(context)
}

/// Build context using full-text search (fallback)
fn build_fts_context(store: &DocumentStore, query: &str) -> Result<String> {
    let results = store.search(query)?;

    if results.is_empty() {
        let all_docs = store.list()?;
        if all_docs.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::new();
        for doc in all_docs.iter().take(3) {
            let preview = truncate_content(&doc.content, 1500);
            context.push_str(&format!(
                "--- Document: {} ---\n{}\n\n",
                doc.filename, preview
            ));
        }
        return Ok(context);
    }

    let mut context = String::new();
    let mut total_chars = 0;
    const MAX_CONTEXT_CHARS: usize = 6000;

    for doc in results.iter().take(5) {
        if total_chars >= MAX_CONTEXT_CHARS {
            break;
        }

        let remaining = MAX_CONTEXT_CHARS - total_chars;
        let preview = truncate_content(&doc.content, remaining.min(2000));

        context.push_str(&format!(
            "--- Document: {} ---\n{}\n\n",
            doc.filename, preview
        ));

        total_chars += preview.len() + doc.filename.len() + 30;
    }

    Ok(context)
}

/// Truncate content to a maximum length, trying to break at sentence boundaries
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    let truncated = &content[..max_len];

    if let Some(pos) = truncated.rfind(". ") {
        return format!("{}.", &truncated[..pos]);
    }

    if let Some(pos) = truncated.rfind("\n\n") {
        return truncated[..pos].to_string();
    }

    if let Some(pos) = truncated.rfind('\n') {
        return truncated[..pos].to_string();
    }

    format!("{}...", truncated)
}
