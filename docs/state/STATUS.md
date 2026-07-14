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
  - Interactive TUI (clap CLI + inquire menu/banner/dashboard, termimad rendering).
- CI pipeline live (`.github/workflows/ci.yml`: check, fmt --check, clippy -D warnings, test on ubuntu; `release.yml` present).
- Onboarded to BEACON Framework.

## In Flight

- BEACON governance/state docs populated and fact-checked against source (this session).
- Production-readiness roadmap defined: 9 epics, v1.0 = E1/E2/E3/E6/E7/E9. See `docs/planning/BACKLOG.md` + `EXECUTION_PLAN.md`.

## Next (v1.0 build — autonomous, parallel agents)

Execution order: foundation (E9 dep upgrades, E7/B-003 fix release, E6 cargo audit) → **E3 first**, then E2, then E1; E6 continuous.

1. **E3 — full-screen ratatui TUI** (B-017/B-018): extract `src/tui/`, multi-pane library/content/status, streaming chat, live search, ingestion dashboard, interactive study modes. *First epic to execute.*
2. **E9 — dependency modernization** (B-024): upgrade lagging crates before/with adding ratatui.
3. **E7 — fix release pipeline** (B-003): binary is `librarian`, not `media-study`.
4. **E2 — pluggable providers** (B-015/B-005/B-016): trait layer, Groq/OpenAI/Anthropic/Ollama, retries, streaming.
5. **E1 — RAG quality** (B-011 structured citations, B-012 rerank, B-001 chunks_fts, B-007 vector index, B-013 chunking, B-014 eval harness).
6. **E6 — testing** (B-002/B-004/B-009), continuous.
