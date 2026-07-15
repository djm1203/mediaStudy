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

All work is on branch **`v1-foundation`**. The prior sessions committed E9/E7-B003/E6-B004/E3/E2 (11
commits, pushed). **This session added E1 + E6 + E7 and left them UNCOMMITTED** (per R-10.5, awaiting
the owner's go-ahead) — all green: build · clippy `--all-targets -D warnings` · fmt · **40 tests**.

**Delivered this session (uncommitted):**
- **E1 — RAG quality (complete):** `chunks_fts` FTS5 keyword arm (B-001), RRF hybrid fusion (B-012),
  structured verifiable citations + TUI "Sources" view (B-011), structure-aware chunking (B-013),
  retrieval eval harness (B-014). ANN index (B-007) evaluated & deferred (D-12).
- **E6:** tests 15 → 40 (B-002); README/`--help` drift fixed (B-009).
- **E7:** `build.rs` version metadata + `install.sh` (B-006); crates.io metadata + `publish.yml` +
  `librarian update` check (B-023).

**New landmarks:** `src/retrieval.rs` (hybrid search + `Citation`), `src/eval.rs` (metrics),
`build.rs` (version), `src/commands/update.rs`. `Message::ChatDone` now carries `Vec<Citation>`.
Product direction unchanged (D-7/D-8/D-9). Roadmap + statuses in `docs/planning/BACKLOG.md`.

**First thing next session:** decide whether to commit the E1/E6/E7 changeset (then push).

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
- **Retrieval** (`src/retrieval.rs`): `hybrid_search()` (semantic cosine + `chunks_fts` FTS5, fused with
  RRF, deduped, top-k) → `Vec<RetrievedChunk>` (carries document_id/chunk_index/filename); `build_context()`
  → numbered `[Source N]` context block + `Vec<Citation>`. `commands::chat::build_grounded_context` wraps
  it (falls back to `build_fts_context` when there are no chunk hits). Skips embedding the query when a
  bucket has no embedded chunks (also what makes it hermetically testable).
- **Eval** (`src/eval.rs`): `evaluate(cases, k)` → hit-rate@k / recall@k / MRR; CI fixture asserts a bar.
- **RAG helpers the TUI reuses** (kept `pub`): `chat::{build_grounded_context,build_fts_context}`,
  `generate::{prompts,get_document_context_pub,parse_qa_pairs}`, `quiz::{QuizQuestion,parse_quiz_questions}`,
  `search::{enhance_query,chunks_overlap}`. Citations thread through `Message::ChatDone { …, citations }`
  into `ChatState::last_citations` → the chat pane's "Sources" view.

## What's Next (resume order)

0. **Commit the uncommitted E1/E6/E7 changeset** (awaiting owner go-ahead per R-10.5), then push.
   The whole v1.0 remainder (E1, E6, E7) is done and green — see "Where We Stopped".
1. **v1.1 — E4 (ingestion robustness):** **B-019** content-hash dedup (skip re-ingest/re-embed of
   identical files), resumable/batch imports, per-file error surfacing, chunked audio for long files;
   **B-008** PDF/OCR robustness for complex/scanned PDFs.
2. **v1.1 — E5 (data safety):** **B-020** bucket export/import + backup/restore + `librarian stats`;
   **B-021** schema migrations + safe re-embed flow (**groundwork done** — `PRAGMA user_version` is now
   the schema-version marker, currently v1; add an ordered migration runner).
3. **v1.1 — E8 (first-run UX):** **B-022** first-run wizard + `librarian doctor` + OS-keychain option;
   **B-010** Homebrew/Scoop/AUR packaging.
4. **B-018** — TUI theme/input polish (configurable accent, mouse, smoother redraws). Needs a visual
   `cargo run` pass (D-11), so pair it with an interactive session.
- **Loose ends:** **B-025** `reqwest 0.13` (deferred: swaps TLS to rustls+aws-lc → cmake/nasm build
  deps); **per-bucket provider override** (provider is global today); a human visual TUI pass + one live
  request per provider before tagging v1.0; Linux arm64 isn't in the release matrix yet.

How to work: autonomous, parallel agents where independent, with a serial contract/refactor step first
when agents would touch shared files. **E1's retrieval items shared `src/retrieval.rs` + the chat path,
so they were done serially by one worker — the right call.** Green gate at every milestone; commit only
on the owner's explicit say-so (R-10.5).

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

- Retrieval uses brute-force cosine over all chunk embeddings (no ANN index) — fine now, watch at large scale (B-007 deferred, D-12).
- The chunk keyword arm is now `chunks_fts` FTS5 (B-001); the old `LIKE` `search_content` remains only as a fallback.
- An LLM provider is a hard dependency for chat/generation and audio transcription; prompts (and transcription audio) leave the machine unless the provider is a local Ollama server (transcription still needs Groq/OpenAI).
- Ingestion feature degradation is silent when FFmpeg/Tesseract are absent.
- **The E1/E6/E7 changeset is uncommitted** — don't lose it; it's the bulk of the v1.0 remainder.
