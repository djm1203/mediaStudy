#![allow(clippy::collapsible_if)]

use anyhow::Result;
use colored::Colorize;
use std::io::Write;
use std::path::PathBuf;

use crate::bucket;
use crate::config::Config;
use crate::embeddings;
use crate::ingest::{ChunkConfig, chunk_text};
use crate::storage::{ChunkStore, Database, DocumentStore};

/// Prompts for different generation types
pub mod prompts {
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

/// Generate a study guide (headless). An empty topic uses all materials.
pub async fn study_guide(topic: Option<String>) -> Result<()> {
    generate_content(
        "Study Guide",
        prompts::STUDY_GUIDE,
        &topic.unwrap_or_default(),
    )
    .await
}

/// Generate flashcards (headless). An empty topic uses all materials.
pub async fn flashcards(topic: Option<String>) -> Result<()> {
    generate_content(
        "Flashcards",
        prompts::FLASHCARDS,
        &topic.unwrap_or_default(),
    )
    .await
}

/// Generate a quiz (headless). An empty topic uses all materials.
pub async fn quiz(topic: Option<String>) -> Result<()> {
    generate_content("Quiz", prompts::QUIZ, &topic.unwrap_or_default()).await
}

/// Generate a summary (headless). An empty topic uses all materials.
pub async fn summary(topic: Option<String>) -> Result<()> {
    generate_content("Summary", prompts::SUMMARY, &topic.unwrap_or_default()).await
}

/// Core generation function
async fn generate_content(name: &str, system_prompt: &str, topic: &str) -> Result<()> {
    let config = Config::load()?;

    let provider = match config.resolve_provider() {
        Ok(provider) => provider,
        Err(e) => {
            println!(
                "{} {} Run {} to set up.",
                "Error:".red().bold(),
                e,
                "librarian config".cyan()
            );
            return Ok(());
        }
    };

    // Get document context
    let context = get_document_context(topic)?;

    if context.is_empty() {
        println!(
            "{} No documents found in current bucket. Add materials first with {}",
            "Error:".red(),
            "librarian add".cyan()
        );
        return Ok(());
    }

    let bucket_name = bucket::get_current_bucket()?
        .map(|b| b.name)
        .unwrap_or_else(|| "(default)".to_string());

    println!("\n{} {}", "Bucket:".dimmed(), bucket_name.cyan());
    println!("{} {}", "Generating:".dimmed(), name.yellow());
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

    // Clear the "Working..." message.
    print!("\r{}\r", " ".repeat(20));
    println!("\n{}", "─".repeat(50).dimmed());
    std::io::stdout().flush().ok();

    match provider.chat(&messages).await {
        Ok(response) => {
            // Render formatted markdown version
            println!("\n{}", "─── Formatted Output ───".dimmed());
            crate::render::render_markdown(&response);
            println!("{}", "─".repeat(50).dimmed());

            // Auto-save to the bucket's generated/ folder and add to the library.
            let filename = format!(
                "{}-{}.md",
                name.to_lowercase().replace(' ', "-"),
                chrono::Local::now().format("%Y%m%d-%H%M")
            );
            let save_path = get_save_path(&filename)?;

            if let Some(parent) = save_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&save_path, &response)?;
            println!(
                "{} Saved to {}",
                "✓".green(),
                save_path.display().to_string().cyan()
            );

            ingest_generated_content(&save_path, &filename, name, &response)?;
            println!("{} Added to your library - now searchable!", "✓".green());

            // Auto-save flashcards/quiz as study items for spaced repetition.
            if name == "Flashcards" || name == "Quiz" {
                save_study_items(name, &response)?;
            }
        }
        Err(e) => {
            println!("{} {}", "Error:".red(), e);
        }
    }

    Ok(())
}

/// Parse generated flashcards/quiz output into study items and save them for
/// spaced repetition (headless — no prompt).
fn save_study_items(content_type: &str, response: &str) -> Result<()> {
    let items = parse_qa_pairs(content_type, response);

    if items.is_empty() {
        return Ok(());
    }

    let db = Database::open()?;
    let store = crate::storage::StudyStore::new(&db);

    let bulk: Vec<(Option<i64>, &str, &str, &str)> = items
        .iter()
        .map(|(item_type, front, back)| (None, item_type.as_str(), front.as_str(), back.as_str()))
        .collect();

    let count = store.bulk_insert(&bulk)?;
    println!(
        "{} Saved {} study items for spaced repetition!",
        "✓".green(),
        count
    );

    Ok(())
}

/// Parse Q/A pairs from generated output
#[allow(clippy::needless_range_loop)]
pub fn parse_qa_pairs(content_type: &str, text: &str) -> Vec<(String, String, String)> {
    let mut items = Vec::new();
    let item_type = if content_type == "Flashcards" {
        "flashcard"
    } else {
        "quiz"
    };

    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Match "Q: ..." / "A: ..." pattern (flashcards)
        if let Some(q) = line.strip_prefix("Q:").or_else(|| line.strip_prefix("Q.")) {
            let question = q.trim().to_string();
            // Look for answer on next lines
            for j in (i + 1)..lines.len().min(i + 4) {
                let ans_line = lines[j].trim();
                if let Some(a) = ans_line
                    .strip_prefix("A:")
                    .or_else(|| ans_line.strip_prefix("A."))
                {
                    items.push((
                        item_type.to_string(),
                        question.clone(),
                        a.trim().to_string(),
                    ));
                    i = j + 1;
                    break;
                }
            }
        }

        // Match numbered questions with **Answer:** pattern
        if line.starts_with(|c: char| c.is_ascii_digit()) {
            // Check for answer a few lines down
            for j in (i + 1)..lines.len().min(i + 8) {
                let ans_line = lines[j].trim().to_lowercase();
                if ans_line.starts_with("**answer") || ans_line.starts_with("answer:") {
                    let q_text = line
                        .trim_start_matches(|c: char| {
                            c.is_ascii_digit() || c == '.' || c == ')' || c == ' '
                        })
                        .to_string();
                    let a_text = lines[j]
                        .trim()
                        .trim_start_matches("**")
                        .trim_start_matches("Answer")
                        .trim_start_matches("answer")
                        .trim_start_matches("**")
                        .trim_start_matches(':')
                        .trim_start_matches("**")
                        .trim()
                        .trim_end_matches("**")
                        .trim()
                        .to_string();

                    if !q_text.is_empty() && !a_text.is_empty() {
                        items.push((item_type.to_string(), q_text, a_text));
                    }
                    i = j + 1;
                    break;
                }
            }
        }

        i += 1;
    }

    items
}

/// Public wrapper for quiz module access
pub fn get_document_context_pub(topic: &str) -> Result<String> {
    get_document_context(topic)
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

    // Dynamic context sizing based on the selected provider's model.
    let config = Config::load()?;
    let max_context_chars = if let Ok(provider) = config.resolve_provider() {
        provider
            .available_context_chars(500, 0, 8192)
            .clamp(2000, 30000)
    } else {
        10000
    };

    let mut context = String::new();
    let mut total_chars = 0;

    for doc in documents.iter().take(10) {
        if total_chars >= max_context_chars {
            break;
        }

        let remaining = max_context_chars - total_chars;
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

    // Dynamic context sizing
    let config = Config::load()?;
    let max_context_chars = if let Ok(provider) = config.resolve_provider() {
        provider
            .available_context_chars(500, 0, 8192)
            .clamp(2000, 30000)
    } else {
        10000
    };

    let mut context = String::new();
    let mut total_chars = 0;

    let similar_ids: Vec<i64> = similar.iter().map(|(id, _)| *id).collect();

    for chunk in &chunks {
        if !similar_ids.contains(&chunk.id) {
            continue;
        }

        if total_chars >= max_context_chars {
            break;
        }

        let doc = doc_store.get(chunk.document_id)?;
        let filename = doc
            .map(|d| d.filename)
            .unwrap_or_else(|| "Unknown".to_string());

        context.push_str(&format!("--- {} ---\n{}\n\n", filename, chunk.content));

        total_chars += chunk.content.len() + filename.len() + 20;
    }

    Ok(context)
}

/// Get the save path for generated content (inside bucket's generated/ folder)
fn get_save_path(filename: &str) -> Result<PathBuf> {
    let base_path = match bucket::get_current_bucket()? {
        Some(bucket) => bucket.path.join("generated"),
        None => {
            // No bucket - save to default data dir
            Config::data_dir()?.join("generated")
        }
    };

    Ok(base_path.join(filename))
}

/// Ingest generated content into the library
fn ingest_generated_content(
    path: &std::path::Path,
    filename: &str,
    content_type: &str,
    content: &str,
) -> Result<()> {
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);

    // Initialize chunks table if needed
    chunk_store.init_schema()?;

    // Check if already exists
    let source_path = path.to_string_lossy().to_string();
    if doc_store.exists_by_path(&source_path)? {
        // Already exists, skip
        return Ok(());
    }

    // Insert document with a special tag
    let doc_type = format!(
        "generated-{}",
        content_type.to_lowercase().replace(' ', "-")
    );
    let doc_id = doc_store.insert(
        &source_path,
        filename,
        &doc_type,
        content,
        Some("generated,study-material"),
    )?;

    // Chunk and embed
    let config = ChunkConfig::default();
    let chunks = chunk_text(content, &config);

    for chunk in &chunks {
        let embedding = embeddings::embed_text(&chunk.text).ok();
        chunk_store.insert(
            doc_id,
            chunk.index as i64,
            &chunk.text,
            embedding.as_deref(),
        )?;
    }

    Ok(())
}
