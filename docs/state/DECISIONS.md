---
title: "DECISIONS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Decisions
author: Derek Martinez
---

# Decisions

## D-1: Rust CLI as the delivery vehicle

**Context:** The tool needs to be fast, local-first, and easy to distribute across platforms.

**Decision:** Build a single-binary Rust CLI (`librarian`, edition 2024) with an interactive
TUI (clap + inquire + termimad).

**Alternatives:** Python/Node service; Electron desktop app.

**Consequences:** Fast startup, no runtime to install, cross-platform binaries; more effort for
some ecosystem integrations vs. Python.

## D-2: Local embeddings for privacy

**Context:** Study materials are personal; users should not have their documents leave the machine
to be indexed.

**Decision:** Embed locally with fastembed (ONNX `all-MiniLM-L6-v2`, 384-dim). Only LLM calls
leave the machine.

**Alternatives:** Hosted embedding APIs (OpenAI, Cohere, Voyage).

**Consequences:** Full-document privacy and offline indexing; ~90MB model download on first run;
CPU embedding cost during ingestion.

## D-3: Groq API for LLM inference

**Context:** Grounded chat and generation (and audio transcription) need a capable LLM at low
latency/cost, without local GPU requirements.

**Decision:** Use the Groq API — `llama-3.3-70b-versatile` (default) / `llama-3.1-8b-instant`
(alt) for chat/generation, `whisper-large-v3-turbo` (default; `whisper-large-v3` for most accurate) for transcription.

**Alternatives:** Local LLM (llama.cpp/Ollama); OpenAI/Anthropic hosted models.

**Consequences:** Fast, cheap inference with no local GPU; hard third-party dependency; prompts
and audio for transcription leave the machine; requires `GROQ_API_KEY`.

## D-4: Per-bucket SQLite databases

**Context:** Users organize materials by "book"/class ("buckets") and want isolation between them.

**Decision:** One SQLite database per bucket via rusqlite (bundled). Tables: documents, chunks,
conversations, study state.

**Alternatives:** Single shared DB with bucket foreign keys; a dedicated vector database.

**Consequences:** Clean isolation and easy per-book backup/delete; bundled SQLite means no external
DB dependency; cross-bucket queries are not a first-class operation.

## D-5: Hybrid retrieval with query enhancement and dedup

**Context:** Pure semantic search misses exact references (exercise/chapter/page numbers); pure
keyword search misses paraphrase.

**Decision:** Combine semantic cosine similarity with keyword matching, plus query enhancement
(`src/search.rs`: filler-prefix/suffix stripping and reference extraction) and Jaccard-based chunk
deduplication (>80% word overlap).

**Alternatives:** Semantic-only; keyword-only; a reranker model.

**Consequences:** Better recall on both paraphrase and exact references; more moving parts to tune.

**Update (2026-07-14, E1):** The keyword arm is now an FTS5 `chunks_fts` index (B-001), and the two
arms are merged with **Reciprocal Rank Fusion** in `src/retrieval.rs` (B-012) rather than
keyword-then-semantic concatenation. The retriever also emits structured, verifiable citations
(document_id + chunk_index, B-011). Retrieval quality is now measurable via the B-014 eval harness.

## D-6: Embeddings stored as BLOBs, brute-force cosine scan

**Context:** At current per-bucket scale, exact nearest-neighbor over all chunks is fast enough.

**Decision:** Serialize embeddings as little-endian f32 BLOBs (`embedding_to_bytes` /
`bytes_to_embedding`) in the chunks table and brute-force cosine-scan them (`find_similar`); no ANN
index.

**Alternatives:** ANN index (HNSW/IVF); a vector DB extension (sqlite-vss).

**Consequences:** Simple, dependency-light, exact results; O(N) per query — revisit if a bucket grows
very large. (Slated for revisit in v1.0 epic E1 / B-007.)

## D-7: Stay a local-first, single-user CLI (product scope)

**Context:** Deciding whether to grow beyond a personal tool toward sync or a hosted multi-user product.

**Decision (2026-07-13):** Keep The Librarian a local-first, single-user CLI — no accounts, no servers,
all data on-device. The production roadmap is polish/robustness/distribution/UX, not backend infra.

**Alternatives:** Local + optional cloud sync; full hosted multi-user product (accounts, web/API).

**Consequences:** Lowest complexity and fastest path to "production ready"; preserves the privacy
guarantee; no multi-device sync until (if ever) revisited in a later major version.

## D-8: Pluggable LLM / transcription providers

**Context:** Chat and Whisper are currently hard-wired to Groq — a single point of failure with no
offline path.

**Decision (2026-07-13):** Introduce `LlmProvider` + `Transcriber` trait seams with adapters for
Groq, OpenAI, Anthropic, and Ollama (local/offline), plus retries/backoff and streaming. Groq stays
the default. (Epic E2 / B-015, B-005, B-016.)

**Alternatives:** Keep Groq-only; add only a local offline model.

**Consequences:** Resilience to provider outages, no vendor lock-in, an offline mode via Ollama; more
abstraction and per-provider adapter maintenance.

## D-9: Full-screen ratatui TUI as the primary interface

**Context:** The interactive experience is line-based menus (inquire + colored); the goal is a
polished, "cool" terminal UI.

**Decision (2026-07-13):** Rebuild the interactive experience as a full-screen ratatui app
(multi-pane library/content/status, streaming chat, live search, ingestion dashboard, interactive
study modes) in a new `src/tui/` module, while keeping `librarian <cmd>` subcommands for scripting.
This is the first v1.0 epic to execute (E3 / B-017, B-018).

**Alternatives:** Polish the existing line-based inquire flow; phased (polish now, ratatui later).

**Consequences:** Biggest visual/UX payoff and a clean structural split from `main.rs`; larger
implementation effort and a new UI dependency (ratatui/crossterm). *(Implemented on `v1-foundation`,
E3 Phases 0–3.)*

## D-10: Enum-dispatched providers (not `dyn`/`async-trait`)

**Context (2026-07-13, E2):** Native `async fn` in traits isn't `dyn`-safe, and the provider set is
small and fixed.

**Decision:** Model providers as `enum Provider { OpenAiCompat(..), Anthropic(..) }` with native
`async fn` methods (`src/llm/provider.rs`) instead of boxed trait objects. One OpenAI-compatible client
covers Groq/OpenAI/Ollama; Anthropic is separate. Providers are constructed via `Config::resolve_provider()`.

**Alternatives:** `async-trait` + `Box<dyn LlmProvider>`; a macro-based dispatch.

**Consequences:** No extra dependency, no boxing, simple matching; adding a provider with a genuinely
different shape means a new enum variant + arms rather than an impl. Fine for a fixed, small set.

## D-11: The full TUI (E3) has no automated regression test

**Context:** A full-screen ratatui app can't be meaningfully exercised by `cargo test` in CI.

**Decision:** Rely on compile + clippy + the pure-logic unit tests for the TUI, and on a human
`cargo run` pass for visual/interaction verification. Keep pure logic (parsing, wrapping, SM-2, RAG
context building) in testable functions outside the render path.

**Consequences:** Rendering/keybinding regressions can slip past CI; a manual smoke pass is part of the
DoD for TUI changes. Consider a headless snapshot-test harness later if churn warrants it.

## D-12: Defer the ANN/vector index; keep brute-force cosine (B-007)

**Context (2026-07-14, E1):** B-007 asked to evaluate an ANN/vector index (sqlite-vec / usearch /
HNSW) to replace the O(N) cosine scan in `embeddings::find_similar`.

**Decision:** **Defer** the ANN index. Keep the brute-force scan for now. The dominant option,
`sqlite-vec`, is a C extension that adds a build-toolchain requirement (raising build-from-source
friction, especially on Windows — the same class of concern that deferred `reqwest 0.13`/B-025), and
the target library scale (OQ-2) is still unanswered by the owner. At current single-user local scale
the scan is fast enough.

**Alternatives:** Adopt `sqlite-vec` now; a pure-Rust in-process HNSW (`instant-distance`/`hnsw_rs`)
to avoid C deps but add index build/persistence complexity.

**Consequences:** No new heavy/platform-specific dependency; retrieval stays simple and exact. Revisit
when OQ-2 is answered or a bucket grows large — the B-014 eval harness makes any swap measurable, and
`user_version` (D-13) gives a migration path for a persisted index.

## D-13: Schema versioning via `PRAGMA user_version`

**Context (2026-07-14):** The `chunks_fts` backfill (B-001) needs a one-shot "has this DB been
migrated?" signal, and `COUNT(*)` on an external-content FTS5 table reads through to the base table so
it can't distinguish an empty index from a full one. The DB otherwise had no schema-version concept
(only `CREATE TABLE IF NOT EXISTS`).

**Decision:** Use SQLite's `PRAGMA user_version` as a lightweight schema-version marker. Version 1 =
`chunks_fts` present and backfilled. `ChunkStore::init_fts` rebuilds the index and bumps the version
when it finds version < 1.

**Alternatives:** A dedicated `schema_meta` table; probing FTS shadow tables; always rebuilding.

**Consequences:** Minimal, idempotent migration gate and the seed for real migrations (E5/B-021). A
single global integer is coarse; formalize an ordered-migration runner if migrations multiply.
