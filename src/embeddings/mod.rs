use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Mutex, OnceLock};

/// Global embedding model instance (loaded once)
static EMBEDDING_MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

/// Get or initialize the embedding model
fn get_model() -> Result<&'static Mutex<TextEmbedding>> {
    if let Some(model) = EMBEDDING_MODEL.get() {
        return Ok(model);
    }

    // Initialize the model
    let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
        .context("Failed to initialize embedding model")?;

    // Try to set it (another thread might have beat us)
    let _ = EMBEDDING_MODEL.set(Mutex::new(model));

    EMBEDDING_MODEL.get().context("Failed to get embedding model")
}

/// Generate embeddings for a list of texts
pub fn embed_texts(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let model = get_model()?;
    let model = model.lock().map_err(|_| anyhow::anyhow!("Failed to lock embedding model"))?;

    let embeddings = model
        .embed(texts.to_vec(), None)
        .context("Failed to generate embeddings")?;

    Ok(embeddings)
}

/// Generate embedding for a single text
pub fn embed_text(text: &str) -> Result<Vec<f32>> {
    let embeddings = embed_texts(&[text])?;
    embeddings
        .into_iter()
        .next()
        .context("No embedding generated")
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// Find the most similar texts given a query embedding
pub fn find_similar(
    query_embedding: &[f32],
    embeddings: &[(i64, Vec<f32>)], // (id, embedding)
    top_k: usize,
) -> Vec<(i64, f32)> {
    let mut scores: Vec<(i64, f32)> = embeddings
        .iter()
        .map(|(id, emb)| (*id, cosine_similarity(query_embedding, emb)))
        .collect();

    // Sort by similarity descending
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Return top k
    scores.into_iter().take(top_k).collect()
}

/// Serialize embedding to bytes for storage
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Deserialize embedding from bytes
pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
