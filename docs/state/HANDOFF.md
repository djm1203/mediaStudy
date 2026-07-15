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

All work is on branch **`v1-foundation`**, now **4 commits ahead of origin (committed, not pushed)** on
top of the prior 11. All green: build · clippy `--all-targets -D warnings` · fmt · **45 tests**. Tree clean.

**This session's commits:**
- `6760871` — **E1** RAG quality (complete) + **E6** tests/docs + **E7** distribution.
- `a85b7b3` — **E4/B-019** content-hash dedup + resilient batch import (core).
- `4669cf3` — **E5/B-020+B-021** stats/export/import/compact/reembed + `meta` table + Windows
  bucket-delete fix.
- `09845b5` — **E8/B-022** `librarian doctor` health check.

**New landmarks:** `src/retrieval.rs` (hybrid search + `Citation`), `src/eval.rs` (metrics),
`src/storage/maintenance.rs` (stats/vacuum/export), `build.rs` (version), `src/commands/{update,data,
doctor}.rs`. `Message::ChatDone` carries `Vec<Citation>`. `documents.content_hash` + a `meta` k/v table
(schema_version + embedding-model identity). Product direction unchanged (D-7/D-8/D-9).

**Verified end-to-end via the real CLI:** `stats`, `export`→`import`→`bucket delete` round-trip, `doctor`,
`--version`. Retrieval/FTS/eval/chunker/providers covered by the 45 tests.

**First thing next session:** consider `git push` (4 commits ahead). Everything left needs the owner's
environment/hands (see What's Next) — the headlessly-verifiable backlog is done.

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

Everything below needs the owner's environment/hands or a decision — the headlessly-verifiable backlog
is complete.

0. **`git push`** the 4 commits (optional; branch is ahead of origin).
1. **Needs real media / external tools (can't verify headlessly):** **B-008** PDF/OCR robustness for
   complex/scanned PDFs; **B-019 remainder** chunked audio + media size/time guards for long recordings.
2. **Needs interactive/visual work:** **B-022 remainder** first-run wizard (TUI onboarding), OS-keychain
   key storage, Windows file-perm hardening; **B-018** TUI theme/input polish (accent, mouse, redraws) —
   verify with `cargo run` (D-11).
3. **Release-artifact-dependent:** **B-010** Homebrew/Scoop/AUR manifests (need real release tarballs +
   checksums); add Linux arm64 to the release matrix.
4. **Owner decisions / deferred:** **B-021** ordered multi-step migration runner (only if migrations
   multiply — `meta.schema_version` + `user_version` groundwork is in); **B-007** ANN index (deferred,
   D-12, pending OQ-2 scale); **B-025** `reqwest 0.13` (deferred — TLS/build-dep change); **per-bucket
   provider override** (global today).
5. **Pre-v1.0 gate:** a human visual TUI pass + one live request per provider (D-11 / R-8).

How to work: autonomous, parallel agents where independent, with a serial contract/refactor step first
when agents touch shared files. Green gate at every milestone; **commit only on the owner's explicit
say-so (R-10.5) with no AI attribution.** This session's commits followed that.

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
