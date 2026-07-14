---
title: "STATUS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Status
author: Derek Martinez
---

# Status — The Librarian

**Last updated:** 2026-07-13T00:00:00Z

## Project Snapshot

- **Product:** The Librarian — a local-first RAG "personal AI study companion" CLI.
- **Crate/binary:** `the-librarian` v0.1.0 / binary `librarian`.
- **Language:** Rust (edition 2024), ~6,725 LOC in `src/`.
- **Build system:** Cargo (`cargo build --release`, LTO + strip).
- **Storage:** SQLite (rusqlite, bundled) — one DB per "bucket" (book/class).
- **Embeddings:** local fastembed ONNX `all-MiniLM-L6-v2` (384-dim); ~90MB model downloaded on first run.
- **LLM:** Groq API — `llama-3.3-70b-versatile` (default) / `llama-3.1-8b-instant` (alt) for chat/generation, `whisper-large-v3-turbo` (default; `whisper-large-v3` for most accurate) for transcription.
- **Optional external tools:** FFmpeg (audio/video), Tesseract (OCR).

## Done

- Core RAG pipeline implemented end to end:
  - Multi-format ingestion: PDF (pdf-extract + lopdf fallback), txt/md, audio/video (Groq Whisper), images (Tesseract OCR), web URLs (scraper/html2text), YouTube.
  - Chunking (~1000 chars, 200 overlap) + local embedding.
  - Hybrid search: semantic cosine similarity + keyword matching, with query enhancement (filler stripping, reference extraction) and Jaccard chunk dedup.
  - Per-bucket SQLite storage (documents, chunks, conversations, study state).
  - Grounded chat over retrieved context (Groq).
  - Study-material generation: study guides, flashcards, quizzes, summaries, homework help.
  - Spaced-repetition review flow.
- CI pipeline live (`.github/workflows/ci.yml`: check, fmt --check, clippy -D warnings, test, **cargo audit** on ubuntu; `release.yml`).
- Onboarded to BEACON Framework; full doc set populated + fact-checked.

## Done this session (branch `v1-foundation`, 11 commits, all green)

- **E9** — dependency modernization: all lagging crates bumped to current majors (`thiserror 2`, `dirs 6`, `rusqlite 0.40`, `colored 3`, `toml 1`, `fastembed 5`, `pdf-extract 0.12`, `lopdf 0.44`, `indicatif 0.18`, `termimad 0.35`, `scraper 0.27`, `html2text 0.17`). `Cargo.lock` now committed.
- **E7/B-003** — fixed `release.yml` (binary `librarian`, `action-gh-release@v2`). **E6/B-004** — `cargo audit` CI job.
- **E3 — full ratatui TUI (DONE):** `src/tui/` with an async event/render loop, `Action`/`Message` plumbing, `Pane` trait + per-screen modules, service/worker layers (all DB/embedding in `spawn_blocking`), and all 9 screens (Home, Chat, Search, Docs, Add/ingest, Study, Quiz, Review, Config). `inquire` and the legacy line-based menu removed; arg-less subcommands open the TUI, arg-provided paths stay headless.
- **E2 — pluggable providers (DONE):** enum-dispatched `Provider` (OpenAI-compat covers Groq/OpenAI/Ollama + Anthropic), retry/backoff, `Config` provider selection + TUI Config-pane selector, generalized `Transcriber` (Groq/OpenAI Whisper). Groq stays default; backward compatible.

## In Flight

- None — stopped at a clean, committed, pushed checkpoint (E3 + E2 done). Session closing.

## Next (resume here — v1.0 remainder, then v1.1)

1. **E1 — RAG quality:** B-011 structured per-chunk citations (chat/study currently do prompt-instructed `[Source: filename]` only), B-012 hybrid reranker (RRF), B-001 `chunks_fts` FTS5 for the keyword arm, B-007 vector index (evaluate `sqlite-vec`), B-013 structure-aware chunking, B-014 retrieval eval harness.
2. **E6 — B-002:** real test coverage for ingest I/O, storage CRUD, embeddings, and the new `provider` adapters (mock HTTP).
3. **E7 — B-006/B-023:** cross-platform prebuilt release binaries + crates.io publish + self-update.
4. **v1.1:** E4 (ingestion robustness), E5 (export/import + schema migrations), E8 (first-run wizard, `doctor`, packaging).
- **Loose ends:** B-009 doc-sync (default model is `openai/gpt-oss-120b` not `llama-3.3-70b-versatile`; update README for the TUI + provider support); B-025 `reqwest 0.13` (deferred — TLS/build-dep change); per-bucket provider override; a human visual pass of the TUI is still advisable.
