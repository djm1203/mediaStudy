---
title: "RISKS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Risks
author: Derek Martinez
---

# Risks — The Librarian

| ID | Risk | Severity | Likelihood | Status | Mitigation |
|----|------|----------|------------|--------|------------|
| R-1 | LLM/transcription depends on a third-party API (availability, key management, cost) and sends prompts + transcription audio off-machine | Medium | Medium | Mitigated | **E2 added pluggable providers (Groq/OpenAI/Anthropic/Ollama) with retry/backoff; Ollama gives a fully offline path.** Data-flow documented (SECURITY_MODEL/DATA_GOVERNANCE); keys via env or config |
| R-8 | The E2 provider adapters and the full ratatui TUI had no automated coverage — only Groq chat was proven end-to-end | Medium | Low | Mitigated (adapters) | **B-002 added tokio mock-HTTP tests for the OpenAI-compat adapter (happy path, 5xx-retry, 4xx error) + pure-logic tests (parse/default_model/ctx_for/Anthropic system-hoist).** TUI still relies on a manual `cargo run` pass (D-11); a live request per provider is still advisable before v1.0 |
| R-2 | Brute-force cosine similarity does not scale to very large libraries (no vector/ANN index) | Medium | Medium | Open (deferred) | Acceptable at current per-bucket scale; ANN index **evaluated and deferred pending OQ-2** (DECISIONS D-12). The B-014 eval harness now measures retrieval quality so a future swap is verifiable |
| R-3 | The chunk-level keyword arm used SQL `LIKE '%kw%'` rather than a dedicated chunk FTS index — a perf/relevance ceiling on that arm | Low | Medium | Resolved | **B-001 promoted the arm to a `chunks_fts` FTS5 index** (ranked `MATCH`, sync triggers, one-time backfill). The retriever fuses it with the semantic arm via RRF (B-012) |
| R-4 | Thin automated test coverage — the ingest I/O pipeline, storage CRUD, embeddings, provider adapters, and command handlers were untested — raising regression risk | Medium | Medium | Mitigated | **B-002 raised coverage 15 → 40 tests** across storage CRUD/FTS, hybrid retrieval + citations, the eval harness, the structure-aware chunker, and provider adapters (mock HTTP). Still untested: embeddings model I/O, media ingest I/O, and the TUI render path (D-11) |
| R-5 | Optional external binaries (FFmpeg, Tesseract) absent → silent feature degradation for audio/video/OCR | Medium | Medium | Open | Detect and clearly message missing binaries; document install steps |
| R-6 | PDF text extraction fragility (pdf-extract) on complex/scanned PDFs | Low | Medium | Open | lopdf fallback already in place; OCR path for scanned pages |
| R-7 | ~90MB embedding model download required on first run — fails offline / on constrained networks | Low | Medium | Open | Document first-run network requirement; consider offline model bundling |
