/// Text chunking strategies for RAG

/// A chunk of text with metadata
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub index: usize,
    pub start_char: usize,
    pub end_char: usize,
}

/// Configuration for chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target size for each chunk in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,  // ~250 tokens
            overlap: 200,      // Some overlap for context continuity
        }
    }
}

/// Split text into chunks with overlap
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let text = text.trim();

    if text.is_empty() {
        return Vec::new();
    }

    // If text is smaller than chunk size, return as single chunk
    if text.len() <= config.chunk_size {
        return vec![Chunk {
            text: text.to_string(),
            index: 0,
            start_char: 0,
            end_char: text.len(),
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while start < text.len() {
        let mut end = (start + config.chunk_size).min(text.len());

        // Ensure end is at a valid UTF-8 character boundary
        end = find_char_boundary(text, end);

        // Try to find a good break point (paragraph, sentence, or word boundary)
        if end < text.len() {
            end = find_break_point(text, start, end);
        }

        let chunk_text = text[start..end].trim().to_string();

        if !chunk_text.is_empty() {
            chunks.push(Chunk {
                text: chunk_text,
                index,
                start_char: start,
                end_char: end,
            });
            index += 1;
        }

        // Move start forward, accounting for overlap
        if end >= text.len() {
            break;
        }

        start = if end > config.overlap {
            find_char_boundary(text, end - config.overlap)
        } else {
            end
        };

        // Make sure we're making progress
        if start >= end {
            start = end;
        }
    }

    chunks
}

/// Find the nearest valid UTF-8 character boundary at or before the given position
fn find_char_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }
    if text.is_char_boundary(pos) {
        return pos;
    }
    // Search backwards for a valid boundary
    let mut p = pos;
    while p > 0 && !text.is_char_boundary(p) {
        p -= 1;
    }
    p
}

/// Find a good break point near the target position
fn find_break_point(text: &str, start: usize, target_end: usize) -> usize {
    // Ensure we're working with valid character boundaries
    let safe_start = find_char_boundary(text, start);
    let safe_end = find_char_boundary(text, target_end);

    if safe_start >= safe_end {
        return safe_end;
    }

    let search_region = &text[safe_start..safe_end];

    // First, try to break at a paragraph boundary
    if let Some(pos) = search_region.rfind("\n\n") {
        if pos > search_region.len() / 2 {
            return safe_start + pos + 2;
        }
    }

    // Then try a sentence boundary
    for ending in [". ", "! ", "? ", ".\n", "!\n", "?\n"] {
        if let Some(pos) = search_region.rfind(ending) {
            if pos > search_region.len() / 3 {
                return safe_start + pos + ending.len();
            }
        }
    }

    // Then try a newline
    if let Some(pos) = search_region.rfind('\n') {
        if pos > search_region.len() / 3 {
            return safe_start + pos + 1;
        }
    }

    // Finally, try a word boundary (space)
    if let Some(pos) = search_region.rfind(' ') {
        return safe_start + pos + 1;
    }

    // Give up and use the target
    safe_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text() {
        let config = ChunkConfig::default();
        let chunks = chunk_text("Hello world", &config);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world");
    }

    #[test]
    fn test_large_text() {
        let config = ChunkConfig {
            chunk_size: 100,
            overlap: 20,
        };
        let text = "A".repeat(500);
        let chunks = chunk_text(&text, &config);
        assert!(chunks.len() > 1);
    }
}
