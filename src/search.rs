use std::collections::HashSet;

/// Filler words/phrases to strip from queries for better embedding search
const FILLER_PREFIXES: &[&str] = &[
    "what is the",
    "what are the",
    "what is",
    "what are",
    "how does the",
    "how does",
    "how do",
    "how is",
    "can you explain",
    "please explain",
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
];

/// Enhance a raw query by stripping filler words for better embedding search
pub fn enhance_query(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('?').trim();
    let lower = trimmed.to_lowercase();

    for prefix in FILLER_PREFIXES {
        if lower.starts_with(prefix) {
            let rest = &trimmed[prefix.len()..].trim_start();
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }

    trimmed.to_string()
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
