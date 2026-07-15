---
title: "CHANGELOG"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Changelog
author: Derek Martinez
---

# Changelog

## 2026-07-14 (E8 — `librarian doctor`, branch `v1-foundation`)

- **E8 / B-022 (partial)** — `librarian doctor`: a read-only environment health check reporting, with
  actionable hints, the configured provider + credential, transcription availability, a writable data
  directory, the embedding model, and the optional FFmpeg/Tesseract tools; exits with a warn/fail tally.
  Verified end-to-end against the real binary. README command list updated. *Remaining for B-022:* the
  interactive first-run wizard, an OS-keychain key option, and Windows file-permission hardening.

## 2026-07-14 (E5 data safety — B-020 + B-021 core, branch `v1-foundation`)

All green: build · clippy `--all-targets -D warnings` · fmt · **45 tests**. New CLI commands verified
end-to-end against the real binary (stats read-only; export→import→delete round-trip).

- **E5 / B-020** — Data management: `src/storage/maintenance.rs` (`gather_stats`, `vacuum`,
  `vacuum_into`) and four headless commands — `librarian stats` (documents/chunks/embedded/study/
  conversations + DB size, warns on an embedding-model mismatch), `librarian export <dest>` (compacted
  portable copy via `VACUUM INTO`), `librarian import <name> <src>` (read-only validates the source is a
  Librarian DB, then copies it into a new bucket), and `librarian compact` (in-place `VACUUM`). Export
  is refused if the destination exists; a `.db` is self-contained so no archive format/dep is needed.
- **E5 / B-021** — Schema/versioning groundwork: a `meta` key/value table seeded with `schema_version`
  (`db::SCHEMA_VERSION`) and the embedding-model identity (`embeddings::{MODEL_ID,DIM}`), kept separate
  from the `chunks_fts` `PRAGMA user_version` gate so a bump can never skip a legacy FTS backfill. New
  `librarian reembed` rebuilds every chunk vector with the current model and stamps the model into
  `meta`; `stats` flags a model mismatch and points at it.
- **Bug fix (Windows)** — `librarian bucket delete` failed with "file in use" (os error 32) because the
  DB connection was still open when `remove_dir_all` ran; the count query is now scoped so the
  connection drops first. Found via the E5 round-trip verification.

## 2026-07-14 (E4 ingestion robustness — B-019 core, branch `v1-foundation`, uncommitted)

All green: build · clippy `--all-targets -D warnings` · fmt · **41 tests**.

- **E4 / B-019 (core)** — **Content-hash dedup**: added a SHA-256 `content_hash` column to `documents`
  (additive `ALTER TABLE` migration guarded by `pragma_table_info`), computed/stored on every insert,
  with `DocumentStore::exists_by_content`. Both ingest paths (headless `commands/add.rs` and the TUI
  `tui/service/add.rs`) now skip byte-identical content re-added under a different path/URL — not just
  same-path duplicates — avoiding wasteful re-embedding. **Batch import hardened**: a single file that
  can't be stat'd/canonicalized is now reported per-file and skipped instead of aborting the whole run.
  New `sha2` dependency + a dedup unit test. *Remaining for B-019:* chunked audio for long recordings
  and media size/time guards (deferred — need real-media verification in an interactive session).

## 2026-07-14 (E1 RAG quality + E6 tests/docs + E7 distribution, branch `v1-foundation`, uncommitted)

All green at every step: build · clippy `--all-targets -D warnings` · fmt · **40 tests** (up from 15).

- **E1 / B-001** — Promoted the chunk keyword arm from SQL `LIKE` to a real `chunks_fts` FTS5
  external-content table with insert/update/delete sync triggers (mirrors `documents_fts`), plus a
  ranked `ChunkStore::search_content_fts` (queries sanitized into a safe `MATCH` expr). Legacy DBs are
  backfilled once via a `PRAGMA user_version` migration gate (`COUNT(*)` on an external-content FTS
  table reads through to the base table, so it can't detect an empty index — the version gate can).
- **E1 / B-012 + B-011** — New `src/retrieval.rs`: unified hybrid search that fuses the semantic
  (cosine) and keyword (FTS5) arms with **Reciprocal Rank Fusion**, replacing the old
  keyword-then-semantic concatenation, and returns `RetrievedChunk`s carrying `document_id` +
  `chunk_index` + `filename`. Answers now get **verifiable structured citations**: the context block
  numbers each source `[Source N: file (chunk X)]`, the system prompt instructs numeric `[Source N]`
  citations, and `Message::ChatDone` threads a `Vec<Citation>` into a **"Sources" view** in the TUI
  chat pane. `commands/chat::build_semantic_context` → `build_grounded_context`.
- **E1 / B-014** — Retrieval **eval harness** (`src/eval.rs`): `EvalCase`/`EvalReport` scoring
  hit-rate@k, recall@k, and MRR over the retriever, with a hermetic keyword-only fixture that gates
  quality in CI (no embedding-model download).
- **E1 / B-013** — **Structure-aware chunking**: `chunk_text` now segments on Markdown headings and
  blank-line paragraphs, packs blocks up to the size target (so chunks land on semantic boundaries),
  carries a word-boundary overlap tail between chunks, and hard-splits oversized blocks.
- **E1 / B-007** — Evaluated an ANN/vector index and **deferred** it (see DECISIONS D-12): keep the
  brute-force cosine scan pending the owner's target-scale answer (OQ-2); avoids adding sqlite-vec's
  C/build-toolchain friction prematurely. The B-014 harness now makes any future swap measurable.
- **E6 / B-002** — Expanded tests from 15 → 40. Added `tempfile` dev-dep and hermetic on-disk-SQLite
  tests: chunks FTS ranking + trigger sync + legacy backfill + hybrid citation metadata; documents
  CRUD + FTS delete-sync + list ordering; retrieval context numbering/budget; eval metrics; the
  structure-aware chunker; and **provider adapters via a real tokio mock-HTTP server** (happy path,
  5xx-retry recovery, non-retryable 4xx error) plus pure logic (`ProviderKind` parse, `default_model`/
  `ctx_for`, Anthropic system-hoisting).
- **E6 / B-009** — Fixed README/`--help` drift: default model `openai/gpt-oss-120b` (not
  `llama-3.3-70b-versatile`), documented the ratatui TUI (replacing the old inquire-menu copy) and the
  pluggable providers, corrected the app dir to `librarian`, updated Models/How-It-Works/Project-
  Structure/Acknowledgments, and removed the non-existent `generate homework` command.
- **E7 / B-006** — Build metadata: `build.rs` embeds git short SHA + commit date into `--version`
  (`librarian 0.1.0 (<sha> <date>)`). Added a one-line `install.sh` (detects OS/arch, pulls the latest
  release binary to `~/.local/bin`). Cross-platform release binaries already attach via B-003.
- **E7 / B-023** — crates.io metadata (`repository`/`homepage`/`readme`/`keywords`/`categories`/
  `exclude`); `cargo package` validated. Added a **`librarian update`** self-update *check* (queries the
  latest GitHub release, compares versions, prints if newer — no binary replacement). Added a manual
  (`workflow_dispatch`-only) `publish.yml` so a release can't auto-publish.
- Seeded schema versioning via `PRAGMA user_version` (DECISIONS D-13) — groundwork for E5/B-021.

## 2026-07-13 (E2 — pluggable providers, branch `v1-foundation`) — E2 COMPLETE

- **E2-core (committed):** enum-dispatched `Provider` abstraction (`src/llm/provider.rs`) — one
  OpenAI-compatible client serves Groq/OpenAI/Ollama, plus an Anthropic Messages-API client, behind
  `chat()`/`chat_stream()`. Retry/backoff + timeouts + clear per-provider errors (B-005). `Config`
  gains `provider`/`openai_api_key`/`anthropic_api_key`/`ollama_url` + `resolve_provider()`/
  `resolve_transcriber()` (Groq/OpenAI Whisper). All LLM call sites + transcription routed through
  the resolved provider. Backward compatible: old configs + `GROQ_API_KEY` work, Groq stays default.
- **E2 Config pane (green, verified):** the TUI Config pane now selects provider (Groq/OpenAI/
  Anthropic/Ollama), edits the per-provider key (or the Ollama URL), and picks a model from that
  provider's suggested list (or a custom id). `ConfigData`/`SaveConfig` extended accordingly; save
  refreshes Home/sidebar. build/clippy `-D warnings`/fmt/15 tests green.
- Not done in E2: per-bucket provider override (global only); `reqwest 0.13` (B-025, deferred —
  changes TLS backend/build deps).

## 2026-07-13 (E3 — ratatui TUI, branch `v1-foundation`)

- **Phase 0 (committed):** scaffolded `src/tui/` — ratatui 0.30 + crossterm 0.29, panic-safe terminal
  setup, async event/render loop, placeholder sidebar|content|status layout; `librarian` (no args)
  launches it. Blueprint saved to `docs/design/TUI_ARCHITECTURE.md`.
- **Phase 1 (green, verified):** the async core — `action`/`message` plumbing, `service`/`worker`
  layers (all DB + embedding work in `spawn_blocking`), theme/keymap/event modules, full `App` state,
  and a working **Chat** vertical slice: builds hybrid RAG context and streams the reply
  token-by-token via a new `GroqClient::chat_stream_tx`, then persists the turn. Exposed the chat
  context builders, generate prompts, and quiz parsers as `pub`. Home + Chat live; other screens are
  placeholders. build/clippy `-D warnings`/fmt/15 tests all green.
- Noted a doc/code drift for B-009: default chat model is `openai/gpt-oss-120b`, not the
  `llama-3.3-70b-versatile` the README/docs claim.
- **Phase 2a (committed):** established the `Pane` trait + `Ctx` so `App` delegates to per-screen
  modules; froze the full `Action`/`Message` contract; split `service.rs` into per-pane modules;
  stubbed the 7 panes as reachable placeholders (number keys 1–9).
- **Phase 2b (green, verified):** implemented all seven panes in parallel (one agent each, isolated
  to `ui/<pane>.rs` + `service/<pane>.rs`) — Search, Docs, Add/ingest (live progress dashboard),
  Study (streamed generation, reusing `chat_stream_tx`), Quiz, Review (SM-2), Config. No shared-file
  edits were needed; integrated build/clippy `-D warnings`/fmt/15 tests all green. The full TUI
  (9 screens) is now functional. Remaining: Phase 3 — route arg-less subcommands into the TUI and
  remove `inquire` + the legacy line-based menu.
- **Phase 3 (green, verified) — E3 COMPLETE:** made the TUI the primary interface. Arg-less
  subcommands (`chat`, `docs`, `config`, `review`, `quiz`, bare `add`/`search`/`delete`/`bucket`/
  `generate`) open the TUI on the right screen; arg-provided paths stay fully headless and
  non-interactive (removed the `inquire` confirm/save/switch prompts). Deleted the legacy line-based
  menu (`run_interactive`, banner/dashboard helpers) and the interactive command entry points;
  `src/commands/{config,review}.rs` removed (the TUI panes replace them). Removed the `inquire`
  dependency entirely (`grep inquire src/ Cargo.toml` empty). build/clippy `-D warnings`/fmt/15 tests
  all green; headless CLI verified non-blocking.

## 2026-07-13 (v1.0 foundation — branch `v1-foundation`, uncommitted)

- **E9 / B-024** — Modernized dependencies to latest majors: `thiserror 2`, `dirs 6`, `rusqlite 0.40`,
  `colored 3`, `toml 1`, `pdf-extract 0.12`, `lopdf 0.44`, `indicatif 0.18`, `fastembed 5`,
  `termimad 0.35`, `scraper 0.27`, `html2text 0.17`. Only breakage was `fastembed 5` (2-line fix in
  `src/embeddings/mod.rs`: `&mut` model borrow + drop redundant `to_vec`). Build + clippy `-D warnings`
  + `cargo fmt --check` clean; 15/15 tests pass. `reqwest 0.13` deferred to B-025 (changes default TLS
  to rustls/aws-lc → new cmake/nasm build deps; belongs with the E2 provider rework). `inquire` left at
  0.7 (removed by E3). Embedding model unchanged (`AllMiniLML6V2`) so stored 384-dim vectors stay valid.
- **E7 / B-003** — Fixed `release.yml`: packaged binary/archive renamed `media-study` → `librarian`;
  bumped `softprops/action-gh-release@v1 → v2`.
- **E6 / B-004** — Added a `cargo audit` security job to CI (`.github/workflows/ci.yml`).

## 2026-07-13 (later — assessment & v1.0 roadmap)

- Verified doc claims against source and corrected four inaccuracies across the doc set: FTS5 is
  real (`documents_fts`) so the README is accurate — only the chunk keyword arm uses `LIKE`; test
  coverage is 5 files / 15 tests (not just `search.rs`); default Whisper model is
  `whisper-large-v3-turbo`; the active app dir is `librarian` (legacy `media-study` auto-migrated).
- Assessed the project and defined a production-readiness roadmap: 9 epics, with v1.0 = E1 (RAG
  quality/citations), E2 (pluggable providers), E3 (ratatui TUI), E6 (tests), E7 (release), E9
  (dependency modernization). Recorded product decisions D-7 (stay local-first CLI), D-8 (pluggable
  providers), D-9 (ratatui TUI). Rewrote `BACKLOG.md` and `EXECUTION_PLAN.md` around the epics.
- Next epic to execute: **E3** (full-screen ratatui TUI), autonomous + parallel agents.

## 2026-07-13

- Populated BEACON governance docs (state, architecture, product, standards, operations,
  compliance, planning) to reflect The Librarian's actual architecture and current state.
- Note: prior feature work (hybrid search, ingestion optimization, chat, local embeddings,
  per-bucket SQLite storage, study-material generation) predates BEACON onboarding and lives
  in git history (commits: "chat update", "added hybrid searching", "optimizing the ingestion",
  "readme update", "final changes").

## 2026-06-06T03:40:25Z

- Project scaffolded and migrated to BEACON Framework
