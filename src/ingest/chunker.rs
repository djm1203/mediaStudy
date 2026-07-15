//! Text chunking strategies for RAG
#![allow(clippy::collapsible_if)]

/// A chunk of text with metadata
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub index: usize,
    #[allow(dead_code)]
    pub start_char: usize,
    #[allow(dead_code)]
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
            chunk_size: 1000, // ~250 tokens
            overlap: 200,     // Some overlap for context continuity
        }
    }
}

/// Split text into chunks (B-013 — structure-aware).
///
/// Instead of a blind sliding window, the text is first segmented into
/// structural **blocks** — paragraphs (blank-line separated) and Markdown
/// headings, which start a fresh block so a section header stays attached to the
/// content that follows it. Blocks are then packed greedily up to
/// `config.chunk_size` so a chunk lands on a semantic boundary rather than
/// mid-sentence, with a trailing-word overlap carried between chunks for
/// continuity. A single block larger than the target is hard-split with the
/// sentence/word break-point heuristics.
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let blocks = segment_blocks(text);

    let mut chunks = Vec::new();
    let mut index = 0;
    let mut buf = String::new();
    let mut buf_start = 0usize;

    for (bstart, btext) in blocks {
        // A block bigger than the whole target gets hard-split on its own.
        if btext.len() > config.chunk_size {
            flush(&mut chunks, &mut index, &mut buf, buf_start);
            for piece in hard_split(&btext, config) {
                let len = piece.len();
                chunks.push(Chunk {
                    text: piece,
                    index,
                    start_char: bstart,
                    end_char: bstart + len,
                });
                index += 1;
            }
            continue;
        }

        let sep = if buf.is_empty() { 0 } else { 2 };
        if !buf.is_empty() && buf.len() + sep + btext.len() > config.chunk_size {
            // Packing this block would overflow — flush and seed the next chunk
            // with an overlap tail of what we just emitted.
            let tail = overlap_tail(buf.trim(), config.overlap);
            flush(&mut chunks, &mut index, &mut buf, buf_start);
            buf.push_str(&tail);
            buf_start = bstart;
        }

        if buf.is_empty() {
            buf_start = bstart;
        } else {
            buf.push_str("\n\n");
        }
        buf.push_str(&btext);
    }

    flush(&mut chunks, &mut index, &mut buf, buf_start);
    chunks
}

/// Emit the accumulated buffer as a chunk (if non-empty) and clear it.
fn flush(chunks: &mut Vec<Chunk>, index: &mut usize, buf: &mut String, buf_start: usize) {
    let text = buf.trim();
    if !text.is_empty() {
        let text = text.to_string();
        let len = text.len();
        chunks.push(Chunk {
            text,
            index: *index,
            start_char: buf_start,
            end_char: buf_start + len,
        });
        *index += 1;
    }
    buf.clear();
}

/// Segment text into structural blocks, returning each block's byte offset in
/// the source and its trimmed text. Blank lines separate paragraphs; a Markdown
/// heading (`#…`) always starts a new block.
fn segment_blocks(text: &str) -> Vec<(usize, String)> {
    let mut blocks: Vec<(usize, String)> = Vec::new();
    let mut cur = String::new();
    let mut cur_start = 0usize;
    let mut offset = 0usize;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        let is_blank = trimmed.is_empty();
        let is_heading = trimmed.starts_with('#');

        if is_blank {
            push_block(&mut blocks, &mut cur, cur_start);
        } else if is_heading {
            push_block(&mut blocks, &mut cur, cur_start);
            cur_start = offset;
            cur.push_str(line);
        } else {
            if cur.is_empty() {
                cur_start = offset;
            }
            cur.push_str(line);
        }
        offset += line.len();
    }
    push_block(&mut blocks, &mut cur, cur_start);
    blocks
}

fn push_block(blocks: &mut Vec<(usize, String)>, cur: &mut String, start: usize) {
    let trimmed = cur.trim();
    if !trimmed.is_empty() {
        blocks.push((start, trimmed.to_string()));
    }
    cur.clear();
}

/// Take the last `overlap` characters of `s` at a word boundary, for continuity
/// between adjacent chunks. Returns empty when `s` is no longer than `overlap`
/// (avoids duplicating an entire short chunk).
fn overlap_tail(s: &str, overlap: usize) -> String {
    if overlap == 0 || s.len() <= overlap {
        return String::new();
    }
    let start = find_char_boundary(s, s.len() - overlap);
    let slice = &s[start..];
    match slice.find(' ') {
        Some(pos) => slice[pos + 1..].to_string(),
        None => slice.to_string(),
    }
}

/// Hard-split an oversized block with sentence/word break points and overlap.
fn hard_split(text: &str, config: &ChunkConfig) -> Vec<String> {
    let text = text.trim();
    if text.len() <= config.chunk_size {
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![text.to_string()]
        };
    }

    let mut out = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let mut end = (start + config.chunk_size).min(text.len());
        end = find_char_boundary(text, end);
        if end < text.len() {
            end = find_break_point(text, start, end);
        }

        let piece = text[start..end].trim().to_string();
        if !piece.is_empty() {
            out.push(piece);
        }

        if end >= text.len() {
            break;
        }
        start = if end > config.overlap {
            find_char_boundary(text, end - config.overlap)
        } else {
            end
        };
        if start >= end {
            start = end;
        }
    }

    out
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

    #[test]
    fn packs_short_paragraphs_together() {
        let config = ChunkConfig {
            chunk_size: 1000,
            overlap: 100,
        };
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_text(text, &config);
        // All three short paragraphs fit under the target → one cohesive chunk.
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("First"));
        assert!(chunks[0].text.contains("Third"));
    }

    #[test]
    fn breaks_on_section_boundary_when_over_budget() {
        let config = ChunkConfig {
            chunk_size: 60,
            overlap: 10,
        };
        let a = "# Section A\nAlpha content for section a here.";
        let b = "# Section B\nBravo content for section b here.";
        let chunks = chunk_text(&format!("{a}\n\n{b}"), &config);
        // The two sections exceed the budget together, so they split — and each
        // heading stays attached to its own section body.
        assert!(chunks.len() >= 2);
        let has_a = chunks
            .iter()
            .any(|c| c.text.contains("Section A") && c.text.contains("Alpha"));
        let has_b = chunks
            .iter()
            .any(|c| c.text.contains("Section B") && c.text.contains("Bravo"));
        assert!(has_a, "section A heading lost its body: {chunks:?}");
        assert!(has_b, "section B heading lost its body: {chunks:?}");
    }

    #[test]
    fn keeps_making_progress_on_huge_unbroken_block() {
        let config = ChunkConfig {
            chunk_size: 50,
            overlap: 10,
        };
        // A single wordless block far larger than the target must still split.
        let text = "x".repeat(300);
        let chunks = chunk_text(&text, &config);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| !c.text.is_empty()));
    }
}
