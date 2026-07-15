//! Retrieval evaluation harness (B-014).
//!
//! Turns retrieval quality into a number so changes are measurable and
//! regressions are caught. An [`EvalCase`] pairs a query with the chunk ids that
//! *should* be retrieved for it; [`evaluate`] runs each case through
//! [`crate::retrieval::hybrid_search`] and aggregates standard IR metrics:
//!
//! - **hit-rate@k** — fraction of cases with at least one relevant chunk in the
//!   top-k,
//! - **recall@k** — mean fraction of a case's relevant chunks found in the top-k,
//! - **MRR** — mean reciprocal rank of the first relevant chunk.
//!
//! The hermetic fixture below exercises the keyword + fusion path (no embedding
//! model, so it runs in CI). Evaluating the full semantic arm needs the local
//! model downloaded; drive [`evaluate`] against a real bucket for that.

#![allow(dead_code)]

use anyhow::Result;

use crate::retrieval::hybrid_search;
use crate::storage::{ChunkStore, DocumentStore};

/// One labelled retrieval example: a query and the chunk ids that are relevant.
pub struct EvalCase {
    pub query: String,
    pub relevant_chunk_ids: Vec<i64>,
}

/// Aggregated retrieval-quality metrics over a set of [`EvalCase`]s.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalReport {
    /// Number of cases scored.
    pub n: usize,
    /// The `k` used for the top-k cutoff.
    pub k: usize,
    /// Fraction of cases with ≥1 relevant chunk in the top-k (0.0–1.0).
    pub hit_rate: f64,
    /// Mean per-case recall@k (0.0–1.0).
    pub recall_at_k: f64,
    /// Mean reciprocal rank of the first relevant chunk (0.0–1.0).
    pub mrr: f64,
}

/// Score `cases` against the current bucket's retriever at cutoff `k`.
pub fn evaluate(
    chunk_store: &ChunkStore,
    doc_store: &DocumentStore,
    cases: &[EvalCase],
    k: usize,
) -> Result<EvalReport> {
    if cases.is_empty() {
        return Ok(EvalReport {
            n: 0,
            k,
            hit_rate: 0.0,
            recall_at_k: 0.0,
            mrr: 0.0,
        });
    }

    let mut hits = 0.0;
    let mut recall_sum = 0.0;
    let mut rr_sum = 0.0;

    for case in cases {
        let retrieved = hybrid_search(chunk_store, doc_store, &case.query, k)?;
        let ids: Vec<i64> = retrieved.iter().map(|c| c.chunk_id).collect();

        let relevant_found = case
            .relevant_chunk_ids
            .iter()
            .filter(|id| ids.contains(id))
            .count();

        if relevant_found > 0 {
            hits += 1.0;
        }
        if !case.relevant_chunk_ids.is_empty() {
            recall_sum += relevant_found as f64 / case.relevant_chunk_ids.len() as f64;
        }
        if let Some(rank) = ids
            .iter()
            .position(|id| case.relevant_chunk_ids.contains(id))
        {
            rr_sum += 1.0 / (rank as f64 + 1.0);
        }
    }

    let n = cases.len() as f64;
    Ok(EvalReport {
        n: cases.len(),
        k,
        hit_rate: hits / n,
        recall_at_k: recall_sum / n,
        mrr: rr_sum / n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    /// Build a small labelled corpus in a hermetic temp DB. Chunks carry no
    /// embeddings, so retrieval runs on the keyword (FTS) arm alone — enough to
    /// guard the keyword + fusion path in CI without the embedding model.
    fn fixture() -> (tempfile::TempDir, Database, Vec<EvalCase>) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open_at_path(dir.path().join("eval.db")).unwrap();
        let doc_id = DocumentStore::new(&db)
            .insert("/src", "biology.md", "text", "content", None)
            .unwrap();
        let store = ChunkStore::new(&db);
        store.init_schema().unwrap();

        let corpus = [
            "Mitochondria generate ATP through cellular respiration",
            "Photosynthesis in chloroplasts converts sunlight to glucose",
            "Ribosomes translate messenger RNA into proteins",
            "The nucleus stores the cell's DNA and controls gene expression",
            "Osmosis is the diffusion of water across a semipermeable membrane",
        ];
        let mut ids = Vec::new();
        for (i, text) in corpus.iter().enumerate() {
            ids.push(store.insert(doc_id, i as i64, text, None).unwrap());
        }

        let cases = vec![
            EvalCase {
                query: "how do mitochondria make ATP".to_string(),
                relevant_chunk_ids: vec![ids[0]],
            },
            EvalCase {
                query: "ribosomes protein synthesis".to_string(),
                relevant_chunk_ids: vec![ids[2]],
            },
            EvalCase {
                query: "water diffusion across a membrane osmosis".to_string(),
                relevant_chunk_ids: vec![ids[4]],
            },
        ];
        (dir, db, cases)
    }

    #[test]
    fn keyword_retrieval_meets_quality_bar() {
        let (_dir, db, cases) = fixture();
        let store = ChunkStore::new(&db);
        let doc_store = DocumentStore::new(&db);
        let report = evaluate(&store, &doc_store, &cases, 3).unwrap();

        assert_eq!(report.n, 3);
        // Every fixture query names words from its target chunk, so the keyword
        // arm alone should find them all and rank them first.
        assert_eq!(report.hit_rate, 1.0, "report: {report:?}");
        assert!(report.recall_at_k > 0.99, "recall too low: {report:?}");
        assert!(report.mrr > 0.8, "mrr too low: {report:?}");
    }

    #[test]
    fn empty_cases_yield_zeroed_report() {
        let (_dir, db, _) = fixture();
        let store = ChunkStore::new(&db);
        let doc_store = DocumentStore::new(&db);
        let report = evaluate(&store, &doc_store, &[], 5).unwrap();
        assert_eq!(report.n, 0);
        assert_eq!(report.mrr, 0.0);
    }
}
