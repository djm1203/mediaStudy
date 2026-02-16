use anyhow::Result;
use colored::Colorize;
use inquire::{Select, Text};

use crate::bucket;
use crate::config::Config;
use crate::embeddings;
use crate::llm::{GroqClient, groq::Message};
use crate::storage::{ChunkStore, ConversationStore, Database, DocumentStore};

const GROUNDED_SYSTEM_PROMPT: &str = r#"You are The Librarian, a knowledgeable study assistant helping a student learn from their course materials.

IMPORTANT INSTRUCTIONS:
1. Answer questions primarily using the provided context from their documents
2. When the context contains relevant information, use it as the foundation for your answer and cite the source
3. If asked about exercises, problems, or questions from the materials, use the textbook knowledge in the context to reason through the answer — guide the student step by step
4. You may use your general knowledge to supplement and explain concepts from the materials, but always prioritize what's in the provided context
5. If the context has no relevant information at all, say so but still try to help using general knowledge, noting that you're going beyond their materials
6. Be thorough in explanations — use examples from the materials when possible

Format citations like: [Source: filename]"#;

const NO_DOCS_SYSTEM_PROMPT: &str = r#"You are The Librarian, a knowledgeable study assistant. The user has no documents loaded in their current library.

Help them by:
1. Answering general questions to the best of your ability
2. Suggesting they add study materials with 'librarian add <file>'
3. Being clear when you're using general knowledge vs. their specific materials"#;

pub async fn run() -> Result<()> {
    let config = Config::load()?;

    let api_key = match config.get_api_key() {
        Some(key) => key,
        None => {
            println!(
                "{} No API key configured. Run {} to set up.",
                "Error:".red().bold(),
                "librarian config".cyan()
            );
            return Ok(());
        }
    };

    let client = GroqClient::new(api_key, config.default_model);

    // Check current bucket and document count
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);
    let conv_store = ConversationStore::new(&db);

    // Initialize chunks table if needed
    chunk_store.init_schema()?;

    let doc_count = doc_store.count()?;
    let chunk_count = chunk_store.count().unwrap_or(0);

    let bucket_name = bucket::get_current_bucket()?
        .map(|b| b.name)
        .unwrap_or_else(|| "(default)".to_string());

    println!();
    println!(
        "    {}",
        "╭──────────────────────────────────────────────────────╮".cyan()
    );
    println!(
        "    {}       {}       {}",
        "│".cyan(),
        "🎓 ASK THE LIBRARIAN 🎓".bold().white(),
        "│".cyan()
    );
    println!(
        "    {}  {}  {}",
        "│".cyan(),
        "Your personal study assistant, ready to help!".dimmed(),
        "│".cyan()
    );
    println!(
        "    {}",
        "├──────────────────────────────────────────────────────┤".cyan()
    );
    println!(
        "    {}  📖 Book: {:<20} 📄 {} docs, {} chunks  {}",
        "│".cyan(),
        bucket_name.cyan(),
        doc_count.to_string().green(),
        chunk_count.to_string().green(),
        "│".cyan()
    );
    println!(
        "    {}  🤖 Model: {:<43} {}",
        "│".cyan(),
        client.model.yellow(),
        "│".cyan()
    );
    println!(
        "    {}",
        "├──────────────────────────────────────────────────────┤".cyan()
    );
    println!(
        "    {}  💡 {} to exit │ Ask anything about your materials!  {}",
        "│".cyan(),
        "quit".yellow().bold(),
        "│".cyan()
    );
    println!(
        "    {}",
        "╰──────────────────────────────────────────────────────╯".cyan()
    );
    println!();

    if doc_count == 0 {
        println!(
            "{} No documents in this bucket. Add some with {}",
            "Note:".yellow(),
            "librarian add <file>".cyan()
        );
        println!("Chat will use general knowledge only.\n");
    } else if chunk_count == 0 {
        println!(
            "{} Documents exist but no chunks/embeddings. Re-add documents to enable semantic search.\n",
            "Note:".yellow()
        );
    }

    // --- Conversation persistence: choose or create conversation ---
    let conversation_id = pick_or_create_conversation(&conv_store)?;
    let mut is_first_message = true;

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

    // Load previous messages if resuming a conversation
    let prev_messages = conv_store.get_messages(conversation_id)?;
    if !prev_messages.is_empty() {
        is_first_message = false;
        println!(
            "{} Loaded {} previous messages.\n",
            "↻".cyan(),
            prev_messages.len()
        );
        for msg in &prev_messages {
            conversation.push(Message {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }
    }

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

        // Auto-title from first user message
        if is_first_message {
            let title: String = input.chars().take(60).collect();
            let title = if let Some(pos) = title.rfind(' ') {
                &title[..pos]
            } else {
                &title
            };
            conv_store.update_title(conversation_id, title)?;
            is_first_message = false;
        }

        // --- Query enhancement for better embedding search ---
        let enhanced_query = crate::search::enhance_query(input);

        // --- Dynamic context sizing ---
        let conversation_chars: usize = conversation.iter().map(|m| m.content.len()).sum();
        let max_context = client
            .available_context_chars(system_prompt.len(), conversation_chars, 4096)
            .clamp(2000, 30000);

        // Search for relevant context using semantic search
        let context = if chunk_count > 0 {
            build_semantic_context(&chunk_store, &doc_store, &enhanced_query, max_context)?
        } else if doc_count > 0 {
            // Fallback to FTS if no chunks
            build_fts_context(&doc_store, input, max_context)?
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
                    content: response.clone(),
                });

                // --- Persist messages ---
                conv_store.add_message(conversation_id, "user", input)?;
                conv_store.add_message(conversation_id, "assistant", &response)?;
            }
            Err(e) => {
                println!("\n{} {}\n", "Error:".red().bold(), e);
                conversation.pop();
            }
        }
    }

    Ok(())
}

/// Let user pick a recent conversation or start a new one
fn pick_or_create_conversation(store: &ConversationStore) -> Result<i64> {
    let recent = store.list_recent(5)?;

    if recent.is_empty() {
        let id = store.create(None)?;
        println!("{} Started new conversation.\n", "✦".cyan());
        return Ok(id);
    }

    let mut options: Vec<String> = recent
        .iter()
        .map(|c| {
            let title = c.title.as_deref().unwrap_or("(untitled)");
            let date = c.updated_at.format("%m/%d %H:%M");
            format!("💬  {} │ {}", title, date)
        })
        .collect();
    options.push("🆕  New conversation".to_string());

    let selection = Select::new("Resume or start new?", options).prompt();

    match selection {
        Ok(s) if s.contains("New conversation") => {
            let id = store.create(None)?;
            println!("{} Started new conversation.\n", "✦".cyan());
            Ok(id)
        }
        Ok(s) => {
            // Find which conversation was selected
            let idx = recent
                .iter()
                .position(|c| {
                    let title = c.title.as_deref().unwrap_or("(untitled)");
                    s.contains(title)
                })
                .unwrap_or(0);
            let conv = &recent[idx];
            println!(
                "{} Resuming: {}\n",
                "↻".cyan(),
                conv.title.as_deref().unwrap_or("(untitled)").bold()
            );
            Ok(conv.id)
        }
        Err(_) => {
            // On cancel, start new
            let id = store.create(None)?;
            Ok(id)
        }
    }
}

/// Build context using hybrid search: semantic (embeddings) + keyword (LIKE) combined
fn build_semantic_context(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    query: &str,
    max_context_chars: usize,
) -> Result<String> {
    // Get all chunks with embeddings for semantic search
    let chunks = chunk_store.get_all_with_embeddings()?;

    if chunks.is_empty() {
        return build_fts_context(doc_store, query, max_context_chars);
    }

    // --- Semantic search: find top 10 similar chunks ---
    let semantic_ids: Vec<i64> = match embeddings::embed_text(query) {
        Ok(query_embedding) => {
            let chunk_embeddings: Vec<(i64, Vec<f32>)> = chunks
                .iter()
                .filter_map(|c| c.embedding.as_ref().map(|e| (c.id, e.clone())))
                .collect();
            let similar = embeddings::find_similar(&query_embedding, &chunk_embeddings, 10);
            similar.iter().map(|(id, _)| *id).collect()
        }
        Err(_) => Vec::new(),
    };

    // --- Keyword search: find chunks containing query terms ---
    let keyword_chunks = chunk_store.search_content(query, 10).unwrap_or_default();
    let keyword_ids: Vec<i64> = keyword_chunks.iter().map(|c| c.id).collect();

    // --- Merge results: keyword hits first (more precise), then semantic ---
    let mut seen = std::collections::HashSet::new();
    let mut merged_ids: Vec<i64> = Vec::new();

    // Keyword results are more precise for specific references (exercise 0.3, page 26, etc.)
    for id in &keyword_ids {
        if seen.insert(*id) {
            merged_ids.push(*id);
        }
    }
    // Then semantic results
    for id in &semantic_ids {
        if seen.insert(*id) {
            merged_ids.push(*id);
        }
    }

    if merged_ids.is_empty() {
        return build_fts_context(doc_store, query, max_context_chars);
    }

    // Collect matched chunks for dedup — from both the loaded chunks and keyword results
    let mut matched_chunks: Vec<(i64, String)> = Vec::new();
    for id in &merged_ids {
        // Try loaded chunks first
        if let Some(c) = chunks.iter().find(|c| c.id == *id) {
            matched_chunks.push((c.id, c.content.clone()));
        } else if let Some(c) = keyword_chunks.iter().find(|c| c.id == *id) {
            matched_chunks.push((c.id, c.content.clone()));
        }
    }

    // Deduplicate chunks with overlapping content
    let deduped = crate::search::deduplicate_chunks(matched_chunks);

    // Build context from deduped chunks
    let mut context = String::new();
    let mut total_chars = 0;

    for (chunk_id, content) in &deduped {
        if total_chars >= max_context_chars {
            break;
        }

        // Find original chunk for metadata — check both sources
        let chunk = chunks.iter().find(|c| c.id == *chunk_id);
        let kw_chunk = keyword_chunks.iter().find(|c| c.id == *chunk_id);
        let (doc_id, chunk_idx) = chunk
            .or(kw_chunk)
            .map(|c| (c.document_id, c.chunk_index))
            .unwrap_or((0, 0));

        let doc = doc_store.get(doc_id)?;
        let filename = doc
            .map(|d| d.filename)
            .unwrap_or_else(|| "Unknown".to_string());

        let remaining = max_context_chars - total_chars;
        let truncated = truncate_content(content, remaining.min(2000));

        context.push_str(&format!(
            "--- Document: {} (chunk {}) ---\n{}\n\n",
            filename, chunk_idx, truncated
        ));

        total_chars += truncated.len() + filename.len() + 50;
    }

    Ok(context)
}

/// Build context using full-text search (fallback) with dynamic sizing
fn build_fts_context(
    store: &DocumentStore,
    query: &str,
    max_context_chars: usize,
) -> Result<String> {
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

    for doc in results.iter().take(5) {
        if total_chars >= max_context_chars {
            break;
        }

        let remaining = max_context_chars - total_chars;
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
