use std::collections::HashSet;

/// Filler words/phrases to strip from queries for better embedding search
const FILLER_PREFIXES: &[&str] = &[
    "can you give me the answer for",
    "can you give me the answer to",
    "can you give me",
    "can you help me with",
    "can you explain",
    "could you explain",
    "could you help me with",
    "i need help with",
    "i need to understand",
    "i want to know about",
    "please help me with",
    "please explain",
    "what is the",
    "what are the",
    "what is",
    "what are",
    "how does the",
    "how does",
    "how do",
    "how is",
    "explain the",
    "explain",
    "tell me about",
    "describe the",
    "describe",
    "define the",
    "define",
    "why does",
    "why is",
    "why do",
    "when does",
    "when is",
    "where does",
    "where is",
    "give me the answer for",
    "give me the answer to",
    "give me",
];

/// Enhance a raw query by stripping filler words for better embedding search.
/// Also extracts specific references (chapter numbers, exercise numbers, page numbers)
/// and includes them as separate search terms.
pub fn enhance_query(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('?').trim();
    let lower = trimmed.to_lowercase();

    // Strip filler prefixes
    let mut cleaned = trimmed.to_string();
    for prefix in FILLER_PREFIXES {
        if lower.starts_with(prefix) {
            let rest = trimmed[prefix.len()..].trim_start();
            if !rest.is_empty() {
                cleaned = rest.to_string();
                break;
            }
        }
    }

    // Strip trailing filler phrases (only from the end of the query)
    let trailing_filler = [
        "and all its sub questions",
        "and all sub questions",
        "and its sub questions",
        "and sub questions",
        "and all the sub questions",
    ];
    let cleaned_lower = cleaned.to_lowercase();
    for suffix in &trailing_filler {
        if cleaned_lower.ends_with(suffix) {
            cleaned = cleaned[..cleaned.len() - suffix.len()].trim().to_string();
        }
    }

    // Strip "specifically" only when it's filler (not followed by useful content)
    let cleaned_lower = cleaned.to_lowercase();
    if let Some(pos) = cleaned_lower.find(" specifically") {
        let after = cleaned[pos + " specifically".len()..].trim();
        if after.is_empty() {
            cleaned = cleaned[..pos].trim().to_string();
        } else {
            // "specifically" is followed by useful content — just remove the word itself
            cleaned = format!("{} {}", cleaned[..pos].trim(), after);
        }
    }

    // Extract specific references (numbers, exercise/chapter/section/page references)
    // These are critical for keyword search
    let specific_refs = extract_references(&cleaned);

    // If we found specific references, append them to help keyword search
    if !specific_refs.is_empty() {
        format!("{} {}", cleaned, specific_refs.join(" "))
    } else {
        cleaned
    }
}

/// Extract specific references like exercise numbers, chapter numbers, page numbers
fn extract_references(query: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let lower = query.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    for (i, word) in words.iter().enumerate() {
        // Look for patterns like "exercise 0.3", "chapter 0", "page 26", "section 1.2"
        let is_ref_keyword = matches!(
            *word,
            "exercise"
                | "exercises"
                | "chapter"
                | "section"
                | "page"
                | "problem"
                | "problems"
                | "question"
                | "questions"
                | "figure"
                | "theorem"
                | "definition"
                | "example"
                | "lemma"
                | "corollary"
                | "proposition"
        );

        if is_ref_keyword
            && let Some(next) = words.get(i + 1)
            && next.chars().any(|c| c.is_ascii_digit())
        {
            refs.push(next.to_string());
        }
    }

    refs
}

/// Check if two text chunks have significant word-level overlap (Jaccard similarity)
pub fn chunks_overlap(a: &str, b: &str, threshold: f64) -> bool {
    let words_a: HashSet<&str> = a
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    let words_b: HashSet<&str> = b
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();

    if words_a.is_empty() || words_b.is_empty() {
        return false;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        return false;
    }

    (intersection as f64 / union as f64) >= threshold
}

/// Remove chunks with >80% word overlap, keeping the first occurrence
pub fn deduplicate_chunks(chunks: Vec<(i64, String)>) -> Vec<(i64, String)> {
    let mut result: Vec<(i64, String)> = Vec::new();

    for (id, content) in chunks {
        let is_dup = result
            .iter()
            .any(|(_, existing)| chunks_overlap(existing, &content, 0.8));
        if !is_dup {
            result.push((id, content));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhance_query_strips_filler() {
        assert_eq!(enhance_query("what is the mitochondria"), "mitochondria");
        assert_eq!(
            enhance_query("explain the process of photosynthesis"),
            "process of photosynthesis"
        );
        assert_eq!(
            enhance_query("how does DNA replication work?"),
            "DNA replication work"
        );
    }

    #[test]
    fn test_enhance_query_preserves_plain() {
        assert_eq!(
            enhance_query("mitochondria function"),
            "mitochondria function"
        );
        assert_eq!(enhance_query("DNA replication"), "DNA replication");
    }

    #[test]
    fn test_enhance_query_exercise_references() {
        let result = enhance_query(
            "can you give me the answer for the chapter 0 exercises specifically 0.3?",
        );
        // Should contain the exercise number for keyword matching
        assert!(result.contains("0.3"));
        assert!(result.contains("chapter 0 exercises"));
    }

    #[test]
    fn test_enhance_query_page_reference() {
        let result = enhance_query("what is on page 26?");
        assert!(result.contains("26"));
    }

    #[test]
    fn test_chunks_overlap_high() {
        let a = "the quick brown fox jumps over the lazy dog";
        let b = "the quick brown fox jumps over the lazy cat";
        assert!(chunks_overlap(a, b, 0.7));
    }

    #[test]
    fn test_chunks_overlap_low() {
        let a = "the quick brown fox";
        let b = "completely different text here";
        assert!(!chunks_overlap(a, b, 0.5));
    }

    #[test]
    fn test_deduplicate_chunks() {
        let chunks = vec![
            (
                1,
                "the quick brown fox jumps over the lazy dog near the river".to_string(),
            ),
            (
                2,
                "the quick brown fox jumps over the lazy dog near the lake".to_string(),
            ), // >80% overlap
            (
                3,
                "completely different content about biology and chemistry".to_string(),
            ),
        ];
        let deduped = deduplicate_chunks(chunks);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].0, 1);
        assert_eq!(deduped[1].0, 3);
    }
}
