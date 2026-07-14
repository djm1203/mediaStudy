---
title: "HANDOFF"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Handoff
author: Derek Martinez
---

# Handoff

## Where We Stopped

All work is on branch **`v1-foundation`** (11 commits ahead of `main`, pushed to origin). This session
delivered, all green (build · clippy `-D warnings` · fmt · 15 tests) and committed:
**E9** (deps modernized), **E7/B-003** (release-pipeline fix), **E6/B-004** (cargo-audit CI job),
**E3** (the full 9-screen ratatui TUI — `inquire` and the legacy menu removed), and **E2** (pluggable
LLM/transcription providers). Product direction (fixed, D-7/D-8/D-9): **local-first CLI, pluggable
providers, full-screen ratatui TUI.** Roadmap + statuses in `docs/planning/BACKLOG.md`.

### Architecture landmarks (read before touching these)
- **TUI** (`src/tui/`): async loop in `mod.rs` (two mpsc channels: `Action` in, `Message` out);
  `app.rs` delegates to per-screen modules via the `Pane` trait (`ui/pane.rs`); `ui/<screen>.rs` +
  `service/<screen>.rs` per pane; `action.rs` holds the `Action`/`Message` contract. **Invariant: all
  `rusqlite`/`fastembed` work runs inside `tokio::task::spawn_blocking` opening its own `Database::open()`
  — never hold a `Connection` across `.await`.** TUI can't be verified by automated tests (needs a real
  terminal) — verify visually with `cargo run`.
- **Providers** (`src/llm/provider.rs`): enum-dispatched `Provider` (no dyn/async-trait). Build one via
  `Config::resolve_provider()`; transcription via `Config::resolve_transcriber()`. `Config` fields:
  `provider`, `groq_api_key`/`openai_api_key`/`anthropic_api_key`, `ollama_url`, `default_model`.
- **RAG helpers the TUI reuses** (kept `pub`): `chat::{build_semantic_context,build_fts_context}`,
  `generate::{prompts,get_document_context_pub,parse_qa_pairs}`, `quiz::{QuizQuestion,parse_quiz_questions}`,
  `search::{enhance_query,deduplicate_chunks}`.

## What's Next (resume order)

1. **E1 — RAG quality:** **B-011** structured per-chunk citations (today it's only a prompt asking the
   model to write `[Source: filename]` — make it verifiable: carry `document_id`+`chunk_index` and show a
   sources view; the chat/study `Message::ChatDone` path is where to thread it). **B-012** hybrid reranker
   (RRF over the semantic + keyword arms). **B-001** add a `chunks_fts` FTS5 table for the keyword arm
   (`chunks.search_content` is `LIKE` today; `documents_fts` already shows the FTS5 pattern). **B-007**
   vector index (evaluate `sqlite-vec`) to replace the brute-force cosine in `embeddings::find_similar`.
   **B-013** structure-aware chunking. **B-014** retrieval eval harness in CI.
2. **E6 — B-002:** tests for ingest I/O, storage CRUD, embeddings, and the `provider` adapters (mock HTTP).
3. **E7 — B-006/B-023:** cross-platform prebuilt binaries on GitHub releases + crates.io + self-update.
4. **v1.1:** E4 (ingestion robustness), E5 (export/import + schema migrations, B-021), E8 (first-run
   wizard, `librarian doctor`, Homebrew/Scoop/AUR).
- **Loose ends:** **B-009** docs/README drift — default chat model is `openai/gpt-oss-120b` (not
  `llama-3.3-70b-versatile`), and the README still describes the old `inquire` menu; **B-025** `reqwest
  0.13` (deferred: it swaps TLS to rustls+aws-lc → cmake/nasm build deps); per-bucket provider override.

How to work: autonomous, parallel agents where independent, with a serial contract/refactor step first
when agents would touch shared files (the pattern used for the TUI panes and the provider layer). Green
gate + commit at every milestone.

## Quick Reference

```bash
# Build (release, LTO + strip)
cargo build --release

# Run tests
cargo test

# Lint (CI treats warnings as errors)
cargo clippy -- -D warnings

# Format
cargo fmt

# Format check (as CI runs it)
cargo fmt --check
```

Runtime: pick a provider in the TUI Config screen (or `provider` + a `*_api_key` in `config.toml`).
Default is Groq (`GROQ_API_KEY` env or `groq_api_key`); OpenAI/Anthropic/Ollama also supported (Ollama
needs no key). Config/data live under the OS "librarian" dir (auto-migrated from legacy "media-study").
Optional binaries: FFmpeg (audio/video), Tesseract (OCR). First run downloads the ~90MB embedding model.
Run the TUI with `cargo run` (no args); `librarian <cmd> <args>` stays headless for scripting.

## What to Watch

- Retrieval uses brute-force cosine over all chunk embeddings (no ANN index) — fine now, watch at large scale.
- The chunk-level keyword arm is `LIKE`-based rather than a dedicated chunk FTS index — a relevance/perf ceiling on that arm only (document-level search already uses FTS5).
- Groq API is a hard dependency for chat/generation and audio transcription; prompts and audio leave the machine.
- Ingestion feature degradation is silent when FFmpeg/Tesseract are absent.
