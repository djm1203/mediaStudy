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

**Last updated:** 2026-07-14T00:00:00Z

## Project Snapshot

- **Product:** The Librarian — a local-first RAG "personal AI study companion" CLI.
- **Crate/binary:** `the-librarian` v0.1.0 / binary `librarian`.
- **Language:** Rust (edition 2024), ~7,600 LOC in `src/`.
- **Build system:** Cargo (`cargo build --release`, LTO + strip); `build.rs` embeds git SHA + date into `--version`.
- **Storage:** SQLite (rusqlite, bundled) — one DB per "bucket" (book/class); `PRAGMA user_version` schema versioning (v1).
- **Embeddings:** local fastembed ONNX `all-MiniLM-L6-v2` (384-dim); ~90MB model downloaded on first run.
- **Retrieval:** hybrid — semantic cosine + `chunks_fts` FTS5 keyword, fused with Reciprocal Rank Fusion; structured citations.
- **LLM:** pluggable provider (Groq default: `openai/gpt-oss-120b`; OpenAI/Anthropic/Ollama also supported), `whisper-large-v3-turbo` (default) for transcription.
- **Optional external tools:** FFmpeg (audio/video), Tesseract (OCR).
- **Tests:** 45 (all green), incl. hermetic on-disk-SQLite + tokio mock-HTTP provider tests.
- **Ingestion:** content-hash (SHA-256) dedup skips byte-identical re-adds; batch import is per-file resilient.
- **Data mgmt:** `librarian {stats,export,import,compact,reembed}`; `meta` table (schema_version + embedding-model identity).

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

## Done previous session (branch `v1-foundation`, 11 commits, all green)

- **E9** — dependency modernization: all lagging crates bumped to current majors (`thiserror 2`, `dirs 6`, `rusqlite 0.40`, `colored 3`, `toml 1`, `fastembed 5`, `pdf-extract 0.12`, `lopdf 0.44`, `indicatif 0.18`, `termimad 0.35`, `scraper 0.27`, `html2text 0.17`). `Cargo.lock` now committed.
- **E7/B-003** — fixed `release.yml` (binary `librarian`, `action-gh-release@v2`). **E6/B-004** — `cargo audit` CI job.
- **E3 — full ratatui TUI (DONE):** `src/tui/` with an async event/render loop, `Action`/`Message` plumbing, `Pane` trait + per-screen modules, service/worker layers (all DB/embedding in `spawn_blocking`), and all 9 screens (Home, Chat, Search, Docs, Add/ingest, Study, Quiz, Review, Config). `inquire` and the legacy line-based menu removed; arg-less subcommands open the TUI, arg-provided paths stay headless.
- **E2 — pluggable providers (DONE):** enum-dispatched `Provider` (OpenAI-compat covers Groq/OpenAI/Ollama + Anthropic), retry/backoff, `Config` provider selection + TUI Config-pane selector, generalized `Transcriber` (Groq/OpenAI Whisper). Groq stays default; backward compatible.

## Done this session (branch `v1-foundation`, all green: build · clippy `--all-targets -D warnings` · fmt · 45 tests)

- **E1 — RAG quality (COMPLETE, committed 6760871):** B-001 `chunks_fts` FTS5 keyword arm (+ backfill),
  B-012 RRF fusion, B-011 structured verifiable citations + a TUI "Sources" view, B-013 structure-aware
  chunking, B-014 retrieval eval harness. B-007 (ANN index) **evaluated & deferred** (D-12, pending OQ-2).
- **E6 (COMPLETE for v1.0, committed 6760871):** B-002 tests 15 → 40; B-009 README/`--help` drift fixed.
- **E7 (COMPLETE for v1.0, committed 6760871):** B-006 `build.rs` version metadata + `install.sh`;
  B-023 crates.io metadata + manual `publish.yml` + `librarian update` self-update check.
- **E4 — ingestion robustness (core, committed a85b7b3):** B-019 content-hash (SHA-256) dedup + resilient
  batch import. Remaining B-019/B-008 (chunked audio, PDF/OCR robustness) need real-media verification.
- **E5 — data safety (core, uncommitted):** B-020 `stats`/`export`/`import`/`compact` + `maintenance.rs`;
  B-021 `meta` table (schema_version + embedding-model identity) + `reembed`. Verified end-to-end via the
  real CLI. Also fixed a Windows `bucket delete` file-lock bug.

## In Flight

- **E5 changeset is uncommitted** (all green). Everything before it (E1/E6/E7 = 6760871, E4 = a85b7b3)
  is committed on `v1-foundation`. Nothing pushed yet.

## Next (resume here)

1. **Commit E5** and consider pushing `v1-foundation` (3–4 commits ahead of origin now).
2. **E8 (first-run UX):** B-022 first-run wizard + `librarian doctor` (doctor is testable); B-010
   Homebrew/Scoop/AUR packaging.
3. **E4/E5 remainders:** chunked audio + PDF/OCR robustness (B-019/B-008 — need real media); an ordered
   migration runner for B-021 if migrations multiply.
4. **B-018** — TUI theme/input polish. Needs a visual `cargo run` pass (D-11).
- **Loose ends:** B-025 `reqwest 0.13` (deferred); per-bucket provider override; a human visual TUI pass
  + a live request per provider before tagging v1.0.
