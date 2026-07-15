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

## OQ-1: Promote chunk keyword search to FTS5? — RESOLVED (2026-07-14)

**Blocking:** No
**Target:** Project owner
**Status:** Resolved — **Yes, done (B-001).**

Resolved by shipping a `chunks_fts` FTS5 external-content table (mirroring `documents_fts`) with
insert/update/delete sync triggers and a one-time backfill, plus a ranked `search_content_fts`. The
hybrid retriever (`src/retrieval.rs`) now fuses this keyword arm with the semantic arm via Reciprocal
Rank Fusion (B-012). The old `LIKE` arm remains as a fallback only.

## OQ-2: Target library scale — do we need an ANN/vector index?

**Blocking:** No
**Target:** Project owner
**Status:** Open

Retrieval currently brute-force cosine-scans every chunk embedding. What per-bucket size do we
expect to support? Above what scale should we introduce an ANN index (HNSW/IVF) or sqlite-vss?

**Note (2026-07-14):** B-007 evaluated this and **deferred** the ANN index pending this answer
(DECISIONS D-12) — `sqlite-vec` adds C/build-toolchain friction and current scale is small. The B-014
eval harness now measures retrieval quality, so a future index swap can be validated against it.

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

What coverage do we want for the ingestion, storage, and retrieval paths?

**Update (2026-07-14, B-002):** coverage is now **40 tests** — storage CRUD + FTS sync (documents &
chunks), hybrid retrieval + structured citations, the B-014 eval harness, the structure-aware chunker,
and the provider adapters via a tokio mock-HTTP server (happy/retry/error) + pure logic. Still
untested: the embedding-model I/O (needs the ~90MB model), media ingest I/O (PDF/OCR/transcription need
external tools/network), and the TUI render path (D-11 — verified manually). Do we want an explicit
coverage target (e.g. a % gate or a required-paths checklist) as a DoD item, and should CI run a
gated integration job that downloads the embedding model?

## OQ-5: Provider defaults & scope (E2 follow-ups)

**Blocking:** No
**Target:** Project owner
**Status:** Open

Provider selection is currently global (in `config.toml` / the TUI Config pane), defaulting to Groq.
(a) Do we want a **per-bucket** provider/model override? (b) Should first run **auto-detect** the
provider from whichever API key is present rather than defaulting to Groq? (c) `reqwest 0.13` (B-025)
was deferred because it swaps the default TLS backend to rustls+aws-lc (adds cmake/nasm build deps) —
do we migrate with an explicit non-aws-lc TLS backend, or stay on 0.12?
