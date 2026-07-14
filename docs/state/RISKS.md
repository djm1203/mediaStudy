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
| R-1 | Groq API is a hard third-party dependency (availability, API-key management, cost) and sends prompts + transcription audio off-machine | High | Medium | Open | Document the dependency and data flow (see SECURITY_MODEL/DATA_GOVERNANCE); support key via env or config; consider a local-LLM fallback path |
| R-2 | Brute-force cosine similarity does not scale to very large libraries (no vector/ANN index) | Medium | Medium | Open | Acceptable at current per-bucket scale; add an ANN index or sqlite-vss if buckets grow large |
| R-3 | The chunk-level keyword arm (`chunks.search_content`) uses SQL `LIKE '%kw%'` rather than a dedicated chunk FTS index — a perf/relevance ceiling on that arm only (document-level search already uses FTS5) | Low | Medium | Open | Optional: promote the chunk keyword arm to a `chunks_fts` FTS5 table for better relevance/perf (tracked in OPEN_QUESTIONS) |
| R-4 | Thin automated test coverage — 15 unit tests cover pure functions in `search.rs`, `ingest/chunker.rs`, `ingest/ocr.rs`, `ingest/url.rs`, and `storage/study.rs`, but the ingest I/O pipeline, storage CRUD, embeddings, LLM/Whisper clients, and command handlers are untested — raising regression risk | Medium | High | Open | Add unit/integration tests for the ingest I/O, storage, embeddings, client, and handler paths |
| R-5 | Optional external binaries (FFmpeg, Tesseract) absent → silent feature degradation for audio/video/OCR | Medium | Medium | Open | Detect and clearly message missing binaries; document install steps |
| R-6 | PDF text extraction fragility (pdf-extract) on complex/scanned PDFs | Low | Medium | Open | lopdf fallback already in place; OCR path for scanned pages |
| R-7 | ~90MB embedding model download required on first run — fails offline / on constrained networks | Low | Medium | Open | Document first-run network requirement; consider offline model bundling |
