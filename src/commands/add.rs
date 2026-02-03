use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Select, Text};
use std::path::Path;

use crate::embeddings;
use crate::ingest::{self, ChunkConfig, ContentType, chunk_text};
use crate::storage::{ChunkStore, Database, DocumentStore};

pub async fn run(path: Option<String>) -> Result<()> {
    let source = match path {
        Some(p) => p,
        None => prompt_for_source()?,
    };

    println!("\n{} {}", "Processing:".dimmed(), source);

    // Check if it's a URL
    if source.starts_with("http://") || source.starts_with("https://") {
        return process_url(&source).await;
    }

    let path = Path::new(&source);

    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", source);
    }

    // Open database
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);

    // Initialize chunks table
    chunk_store.init_schema()?;

    if path.is_dir() {
        process_directory(path, &doc_store, &chunk_store).await?;
    } else {
        process_file(path, &doc_store, &chunk_store).await?;
    }

    Ok(())
}

fn prompt_for_source() -> Result<String> {
    let options = vec!["File", "Directory", "URL/Website", "YouTube Video"];

    let source_type = Select::new("What would you like to add?", options).prompt()?;

    let (prompt_text, help_text) = match source_type {
        "File" => ("Enter file path:", "You can use tab for path completion"),
        "Directory" => (
            "Enter directory path:",
            "You can use tab for path completion",
        ),
        "URL/Website" => ("Enter URL:", "https://example.com/article"),
        "YouTube Video" => ("Enter YouTube URL:", "https://youtube.com/watch?v=..."),
        _ => unreachable!(),
    };

    let path = Text::new(prompt_text)
        .with_help_message(help_text)
        .prompt()?;

    Ok(path)
}

fn content_type_str(ct: &ContentType) -> &'static str {
    match ct {
        ContentType::Pdf => "pdf",
        ContentType::Text => "text",
        ContentType::Markdown => "markdown",
        ContentType::Audio => "audio",
        ContentType::Video => "video",
        ContentType::Image => "image",
        ContentType::Url => "url",
        ContentType::Unknown => "unknown",
    }
}

/// Create a spinner for indeterminate progress
fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner
}

/// Create a progress bar for determinate progress
fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:30.cyan/dim}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("━━─"),
    );
    pb.set_message(message.to_string());
    pb
}

async fn process_file(
    path: &Path,
    doc_store: &DocumentStore<'_>,
    chunk_store: &ChunkStore<'_>,
) -> Result<()> {
    let abs_path = tokio::fs::canonicalize(path).await?;
    let source_path = abs_path.to_string_lossy().to_string();

    // Check if already exists
    if doc_store.exists_by_path(&source_path)? {
        println!(
            "{} Document already exists: {}",
            "⚠".yellow(),
            path.display()
        );
        return Ok(());
    }

    // Check if this is a media file that needs transcription
    let is_media = ingest::requires_transcription(path);

    let spinner = if is_media {
        create_spinner("Transcribing audio/video...")
    } else {
        create_spinner("Extracting content...")
    };

    // Use async extraction for all files (handles both media and non-media)
    let content = ingest::extract_from_file_async(path).await?;
    spinner.finish_and_clear();

    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Insert document into database
    let doc_id = doc_store.insert(
        &source_path,
        &filename,
        content_type_str(&content.content_type),
        &content.text,
        None,
    )?;

    // Chunk the document
    let config = ChunkConfig::default();
    let chunks = chunk_text(&content.text, &config);
    let num_chunks = chunks.len();

    // Progress bar for embedding
    let pb = create_progress_bar(num_chunks as u64, "Embedding chunks");

    // Generate embeddings and store chunks
    for chunk in &chunks {
        // Generate embedding
        let embedding = embeddings::embed_text(&chunk.text).ok();

        chunk_store.insert(
            doc_id,
            chunk.index as i64,
            &chunk.text,
            embedding.as_deref(),
        )?;

        pb.inc(1);
    }

    pb.finish_and_clear();

    let preview_len = content.text.len().min(200);
    let preview = &content.text[..preview_len];

    println!("{}", "─".repeat(50).dimmed());
    println!("{} {:?}", "Type:".bold(), content.content_type);
    println!("{} {} chars", "Length:".bold(), content.text.len());
    println!("{} {}", "Chunks:".bold(), num_chunks);
    println!("{} {}", "ID:".bold(), doc_id);
    println!("{}", "Preview:".bold());
    println!(
        "{}{}",
        preview.dimmed(),
        if content.text.len() > 200 { "..." } else { "" }
    );
    println!("{}", "─".repeat(50).dimmed());

    println!(
        "\n{} Added {} (id: {}, {} chunks)",
        "✓".green(),
        filename,
        doc_id,
        num_chunks
    );

    Ok(())
}

async fn process_directory(
    path: &Path,
    doc_store: &DocumentStore<'_>,
    chunk_store: &ChunkStore<'_>,
) -> Result<()> {
    // First, collect all files to get total count
    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_path = entry.path();
        let metadata = tokio::fs::metadata(&file_path).await?;
        if metadata.is_file() {
            files.push(file_path);
        }
    }

    if files.is_empty() {
        println!("{} No files found in directory", "⚠".yellow());
        return Ok(());
    }

    let total_files = files.len();
    println!("Found {} files\n", total_files);

    let pb = create_progress_bar(total_files as u64, "Processing files");

    let mut count = 0;
    let mut errors = 0;
    let mut skipped = 0;
    let mut total_chunks = 0;
    #[allow(clippy::type_complexity)]
    let mut results: Vec<(String, Result<(usize, usize), String>)> = Vec::new();

    for file_path in files {
        let abs_path = tokio::fs::canonicalize(&file_path).await?;
        let source_path = abs_path.to_string_lossy().to_string();

        let filename_display = file_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        pb.set_message(format!("Processing: {}", filename_display));

        // Check if already exists
        if doc_store.exists_by_path(&source_path)? {
            results.push((filename_display, Err("already exists".to_string())));
            skipped += 1;
            pb.inc(1);
            continue;
        }

        match ingest::extract_from_file_async(&file_path).await {
            Ok(content) => {
                let filename = file_path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                match doc_store.insert(
                    &source_path,
                    &filename,
                    content_type_str(&content.content_type),
                    &content.text,
                    None,
                ) {
                    Ok(doc_id) => {
                        // Chunk and embed
                        let config = ChunkConfig::default();
                        let chunks = chunk_text(&content.text, &config);
                        let num_chunks = chunks.len();

                        for chunk in &chunks {
                            let embedding = embeddings::embed_text(&chunk.text).ok();
                            let _ = chunk_store.insert(
                                doc_id,
                                chunk.index as i64,
                                &chunk.text,
                                embedding.as_deref(),
                            );
                        }

                        results.push((filename, Ok((content.text.len(), num_chunks))));
                        count += 1;
                        total_chunks += num_chunks;
                    }
                    Err(e) => {
                        results.push((filename_display, Err(format!("db error: {}", e))));
                        errors += 1;
                    }
                }
            }
            Err(e) => {
                results.push((filename_display, Err(e.to_string())));
                errors += 1;
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    // Print results
    println!("\n{}", "Results:".bold());
    println!("{}", "─".repeat(60).dimmed());

    for (filename, result) in results {
        match result {
            Ok((chars, chunks)) => {
                println!(
                    "  {} {} ({} chars, {} chunks)",
                    "✓".green(),
                    filename,
                    chars,
                    chunks
                );
            }
            Err(ref e) if e == "already exists" => {
                println!("  {} {} ({})", "⊘".yellow(), filename, e);
            }
            Err(e) => {
                println!("  {} {} ({})", "✗".red(), filename, e);
            }
        }
    }

    println!("{}", "─".repeat(60).dimmed());
    println!(
        "\n{} {} added ({} chunks), {} skipped, {} errors",
        "Summary:".bold(),
        count,
        total_chunks,
        skipped,
        errors
    );

    Ok(())
}

async fn process_url(url: &str) -> Result<()> {
    // Open database
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);

    // Initialize chunks table
    chunk_store.init_schema()?;

    // Check if already exists
    if doc_store.exists_by_path(url)? {
        println!("{} URL already exists in database: {}", "⚠".yellow(), url);
        return Ok(());
    }

    // Determine if it's a YouTube URL for display
    let is_youtube = url.contains("youtube.com") || url.contains("youtu.be");

    let spinner = if is_youtube {
        create_spinner("Fetching YouTube transcript...")
    } else {
        create_spinner("Fetching and parsing URL...")
    };

    // Fetch and extract content
    let content = ingest::fetch_url(url).await?;
    spinner.finish_and_clear();

    // Insert document
    let content_type = if is_youtube { "youtube" } else { "url" };
    let doc_id = doc_store.insert(url, &content.title, content_type, &content.text, None)?;

    // Chunk and embed
    let config = ChunkConfig::default();
    let chunks = chunk_text(&content.text, &config);
    let num_chunks = chunks.len();

    let pb = create_progress_bar(num_chunks as u64, "Embedding chunks");

    for chunk in &chunks {
        let embedding = embeddings::embed_text(&chunk.text).ok();
        chunk_store.insert(
            doc_id,
            chunk.index as i64,
            &chunk.text,
            embedding.as_deref(),
        )?;
        pb.inc(1);
    }

    pb.finish_and_clear();

    let preview_len = content.text.len().min(200);
    let preview = &content.text[..preview_len];

    println!("{}", "─".repeat(50).dimmed());
    println!("{} {}", "Title:".bold(), content.title);
    println!("{} {}", "Type:".bold(), content_type);
    println!("{} {} chars", "Length:".bold(), content.text.len());
    println!("{} {}", "Chunks:".bold(), num_chunks);
    println!("{} {}", "ID:".bold(), doc_id);
    println!("{}", "Preview:".bold());
    println!(
        "{}{}",
        preview.dimmed(),
        if content.text.len() > 200 { "..." } else { "" }
    );
    println!("{}", "─".repeat(50).dimmed());

    println!(
        "\n{} Added \"{}\" (id: {}, {} chunks)",
        "✓".green(),
        content.title,
        doc_id,
        num_chunks
    );

    Ok(())
}
