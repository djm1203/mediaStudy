---
title: "OPEN QUESTIONS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: OpenQuestions
author: Derek Martinez
---

# Open Questions — The Librarian

## OQ-1: Promote chunk keyword search to FTS5?

**Blocking:** No
**Target:** Project owner
**Status:** Open

Document-level full-text search already uses a real FTS5 virtual table (`documents_fts`, created in
`src/storage/db.rs:71` with sync triggers, queried via `MATCH` in `src/storage/documents.rs:88`), so
the README's FTS5 description is accurate. The one nuance: the hybrid retriever's chunk-level keyword
arm `src/storage/chunks.rs::search_content` uses SQL `LIKE '%kw%'` rather than a dedicated chunk FTS
index. Do we promote that arm to its own `chunks_fts` FTS5 table (mirroring `documents_fts`, with sync
triggers) for better keyword relevance/perf at scale? This is an enhancement decision, not a doc/code
contradiction.

## OQ-2: Target library scale — do we need an ANN/vector index?

**Blocking:** No
**Target:** Project owner
**Status:** Open

Retrieval currently brute-force cosine-scans every chunk embedding. What per-bucket size do we
expect to support? Above what scale should we introduce an ANN index (HNSW/IVF) or sqlite-vss?

## OQ-3: Release and distribution plan

**Blocking:** No
**Target:** Project owner
**Status:** Open

Do we ship prebuilt binaries per platform (via `release.yml`), and for which targets
(Windows/macOS/Linux)? How are the FFmpeg/Tesseract/model prerequisites communicated to users?

## OQ-4: Test-coverage targets

**Blocking:** No
**Target:** Project owner
**Status:** Open

What coverage do we want for the ingestion, storage, and retrieval paths? Today 15 unit tests cover
pure functions in `src/search.rs`, `ingest/chunker.rs`, `ingest/ocr.rs`, `ingest/url.rs`, and
`storage/study.rs` (SM-2 scheduler); the ingest I/O pipeline, storage CRUD, embeddings, the provider
adapters, and the TUI are untested. Which paths are highest priority to cover first? (E2 added
OpenAI/Anthropic/Ollama adapters that have not been run against live APIs — mock-HTTP tests are B-002.)

## OQ-5: Provider defaults & scope (E2 follow-ups)

**Blocking:** No
**Target:** Project owner
**Status:** Open

Provider selection is currently global (in `config.toml` / the TUI Config pane), defaulting to Groq.
(a) Do we want a **per-bucket** provider/model override? (b) Should first run **auto-detect** the
provider from whichever API key is present rather than defaulting to Groq? (c) `reqwest 0.13` (B-025)
was deferred because it swaps the default TLS backend to rustls+aws-lc (adds cmake/nasm build deps) —
do we migrate with an explicit non-aws-lc TLS backend, or stay on 0.12?
