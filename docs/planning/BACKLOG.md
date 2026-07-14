---
title: "BACKLOG"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Backlog
author: Derek Martinez
---

# Backlog — the-librarian

## Tagging guide

This is a **single-solution** repo (crate `the-librarian`, binary `librarian`), so the `[SOL-A]` multi-solution scheme does not apply. Tag every item with:

- `[CORE]` — touches the core RAG / CLI pipeline (ingestion, embeddings, retrieval, storage, chat, generation)
- `[TUI]` — touches the interactive terminal UI
- `[ALL]` — cross-cutting (CI, docs, tooling, release, dependencies)
- No tag = incomplete item, do not start work

**Status values:** `Pending` → `In Progress` → `Shipped` | `Blocked`

When an item ships, update its Status to `Shipped — <commit hash> — <date>`.

**Rollout Risk values** (this is a local-first, single-user CLI; "records" = a user's local library, so most changes carry little rollout risk):
- `None` — no effect on existing local data or behavior
- `Low` — additive or invisible until opted in; may require a one-time local re-index
- `Medium` — changes something users will notice, or requires migrating existing local databases
- `High` — breaks existing local databases or CLI contracts without a migration path

---

## Product direction (decided 2026-07-13)

The Librarian stays a **local-first, single-user CLI** — no accounts, no servers, all data on-device. See `docs/state/DECISIONS.md` (D-7/D-8/D-9). The road to a production-ready v1.0 is organized into nine epics; six are in the v1.0 cut.

| Epic | Theme | In v1.0? | Backlog items |
|------|-------|----------|---------------|
| **E1** | RAG quality & trust (structured citations, vector index, FTS5 rerank, chunking, eval) | ✅ | B-001, B-007, B-011, B-012, B-013, B-014 |
| **E2** | Pluggable LLM / transcription providers (trait layer, Groq/OpenAI/Anthropic/Ollama, retries, streaming, offline) | ✅ | B-005, B-015, B-016 |
| **E3** | Full-screen TUI (ratatui) — **first epic to execute** | ✅ | B-017, B-018 |
| **E6** | Testing & quality gates | ✅ | B-002, B-004, B-009 |
| **E7** | Distribution & release | ✅ | B-003, B-006, B-023 |
| **E9** | Dependency & toolchain modernization | ✅ | B-024 |
| **E4** | Ingestion robustness & reach | v1.1 | B-008, B-019 |
| **E5** | Data safety & management (export/import, migrations, backup) | v1.1 | B-020, B-021 |
| **E8** | Config, secrets & first-run UX | v1.1 | B-022, B-010 |

Execution order (autonomous, parallel agents where independent): **E3 → E2 → E1**, with **E9** + **E7 (B-003)** done first as foundation and **E6** running continuously alongside every epic. See `docs/planning/EXECUTION_PLAN.md`.

---

## Active items

| ID | Solution | Title | Priority | Rollout Risk | Status |
|----|----------|-------|----------|--------------|--------|
| B-017 | [TUI] | **E3** — Rebuild the interactive experience as a full-screen **ratatui** TUI: multi-pane layout (library sidebar, main content pane, status bar), streaming chat view with rendered markdown, live incremental search, ingestion progress dashboard, and interactive flashcard/quiz/review modes. Split a new `src/tui/` module out of `main.rs`; keep `librarian <cmd>` subcommands for scripting. Blueprint: `docs/design/TUI_ARCHITECTURE.md` | HIGH | Low | Done — `v1-foundation`. Phases 0–3 complete: scaffold, async core + streaming Chat, all 7 panes, and the cutover (arg-less subcommands open the TUI; `inquire` + legacy menu removed; `commands/{config,review}.rs` deleted). Full 9-screen TUI is the primary interface |
| B-018 | [TUI] | **E3** — Cohesive theme + input system for the TUI: light/dark + configurable accent, vim-style + arrow keybindings, mouse support, help overlay, smooth redraws | MEDIUM | None | Pending |
| B-024 | [ALL] | **E9** — Dependency & toolchain modernization: upgrade lagging crates (`thiserror 1→2`, `dirs 5→6`, `rusqlite 0.31→0.40`, `colored 2→3`, `toml 0.8→1`, `pdf-extract 0.12`, `lopdf 0.44`, `indicatif 0.18`, `fastembed 4→5`, `termimad 0.35`, `scraper 0.27`, `html2text 0.17`); build/clippy/tests green with a 2-line `fastembed 5` fix. `reqwest 0.13` deferred to B-025; `inquire` left (removed by E3). Add `cargo outdated`/Dependabot next | MEDIUM | Low | In Progress — `v1-foundation` (uncommitted) |
| B-003 | [ALL] | **E7** — Fix the release pipeline: `release.yml` packaged a `media-study` binary but the real binary is `librarian`; fixed binary + archive names and bumped `action-gh-release@v1→v2` | HIGH | None | Done — `v1-foundation` (uncommitted) |
| B-015 | [CORE] | **E2** — Introduce `LlmProvider` + `Transcriber` traits and adapters for Groq, OpenAI, Anthropic, and **Ollama** (local/offline). Provider + model configurable per bucket and globally; auto-detect from available keys; offline mode when Ollama is present | HIGH | Low | Pending |
| B-005 | [CORE] | **E2** — Retry + exponential backoff, timeout and rate-limit handling, and clearer errors for all provider calls (chat, generation, transcription) | MEDIUM | None | Pending |
| B-016 | [CORE] | **E2** — Streaming responses in chat/generation (token streaming through the provider trait into the TUI chat view) | MEDIUM | None | Pending |
| B-025 | [ALL] | **E2** — Migrate `reqwest 0.12 → 0.13` as part of the provider-layer rework. Deferred from E9 because reqwest 0.13 changes the default TLS backend to `rustls`+`aws-lc-rs`, which adds a cmake/C-toolchain (and nasm on Windows) build requirement; choose the TLS backend explicitly (e.g. `rustls-tls-native-roots` or keep `native-tls`) to avoid raising build-from-source friction | MEDIUM | Low | Pending |
| B-011 | [CORE] | **E1** — Structured, verifiable citations: upgrade from prompt-instructed inline `[Source: filename]` (`chat.rs:26`) to guaranteed citations carrying `document_id` + `chunk_index`, rendered as a browsable "sources" view in the TUI | HIGH | Low | Pending |
| B-012 | [CORE] | **E1** — Proper hybrid reranking (e.g. Reciprocal Rank Fusion or weighted score fusion) to merge the semantic + keyword arms, replacing simple concatenation | MEDIUM | Low | Pending |
| B-001 | [CORE] | **E1** — Promote the chunk-level keyword arm from SQL `LIKE` to a dedicated `chunks_fts` FTS5 table (sync triggers mirroring `documents_fts`) for better keyword relevance/perf. Document-level search already uses FTS5. See `src/storage/chunks.rs:151` | MEDIUM | Low | Pending |
| B-007 | [CORE] | **E1** — ANN / vector index for scale: retrieval currently loads all chunk embeddings and brute-force O(n) cosine-scans them (`get_all_with_embeddings`). Evaluate `sqlite-vec` / `usearch` / HNSW; query top-k without loading the whole set | MEDIUM | Low | Pending |
| B-013 | [CORE] | **E1** — Smarter, structure-aware chunking (per-format, semantic boundaries) with tunable overlap, replacing the fixed 1000/200 split | LOW | Low | Pending |
| B-014 | [CORE] | **E1** — Retrieval **eval harness**: fixture queries → expected chunks, scored in CI so retrieval-quality changes are measurable and regressions are caught | MEDIUM | None | Pending |
| B-002 | [CORE] | **E6** — Expand automated test coverage. Pure-function tests already cover `search.rs`, `ingest/chunker.rs`, `ingest/ocr.rs`, `ingest/url.rs`, and `storage/study.rs` (SM-2). Add unit/integration tests for the ingest I/O pipeline, storage CRUD, embeddings, provider adapters (mocked HTTP), and command handlers | HIGH | None | Pending |
| B-004 | [ALL] | **E6** — Add `cargo audit` (dependency vulnerability scan) as a CI job; consider `cargo deny` for licenses | MEDIUM | None | Done (CI job added) — `v1-foundation` (uncommitted); `cargo deny` still optional |
| B-009 | [ALL] | **E6** — Keep README/docs in sync with implementation as an ongoing DoD check. Known drift to fix: the default chat model is `openai/gpt-oss-120b` (`src/llm/groq.rs:88`), but the README and several BEACON docs say `llama-3.3-70b-versatile`; also document the ratatui TUI once it lands | MEDIUM | None | Pending |
| B-006 | [ALL] | **E7** — Cross-platform prebuilt binaries (Linux/macOS incl. Apple Silicon/Windows) actually attach to GitHub releases; one-line install script; embed version/build metadata | MEDIUM | None | Pending |
| B-023 | [ALL] | **E7** — Publish to crates.io (`cargo install the-librarian`) and add a `self-update` check | MEDIUM | None | Pending |
| B-008 | [CORE] | **E4** (v1.1) — Improve PDF/OCR extraction robustness for complex/scanned PDFs (extend the `pdf-extract` + `lopdf` fallback; OCR fallback for scanned pages) | LOW | None | Pending |
| B-019 | [CORE] | **E4** (v1.1) — Content-hash dedup (skip re-ingesting/re-embedding identical files), resumable/batch imports, per-file error surfacing, size/time guards + chunked audio for long recordings | MEDIUM | Low | Pending |
| B-020 | [CORE] | **E5** (v1.1) — Bucket export/import (portable archive) + backup/restore; `librarian stats`; vacuum/compact | MEDIUM | Low | Pending |
| B-021 | [CORE] | **E5** (v1.1) — Schema versioning + migrations (today it is `CREATE TABLE IF NOT EXISTS` only); safe re-embed flow when the embedding model changes | MEDIUM | Medium | Pending |
| B-022 | [ALL] | **E8** (v1.1) — First-run wizard (pick provider, key, download model with progress), `librarian doctor` health check, OS-keychain option for keys (file fallback), Windows file-perm hardening | MEDIUM | Low | Pending |
| B-010 | [ALL] | **E8** (v1.1) — Homebrew formula / Scoop manifest / AUR package | LOW | None | Pending |

---

## Shipped / done (predates BEACON — see git history)

These are already implemented and working; captured here so the backlog reflects true project state.

| Area | Status |
|------|--------|
| Multi-format ingestion (PDF, txt/md, audio/video via FFmpeg+Groq Whisper, image OCR via Tesseract, web URLs, YouTube) | Shipped |
| Chunking (1000-char chunks, 200-char overlap) | Shipped |
| Local embeddings (fastembed all-MiniLM-L6-v2, 384-dim) | Shipped |
| Hybrid retrieval (semantic cosine + keyword + query enhancement + Jaccard dedup) | Shipped |
| Per-bucket SQLite storage (`documents_fts` FTS5 for document-level search) | Shipped |
| Interactive chat with grounded responses + prompt-instructed inline source tags (`[Source: filename]`) | Shipped — see B-011 for structured-citation upgrade |
| Study-material generation (study guide / flashcards / quiz / summary / homework) | Shipped |
| Spaced-repetition review (SM-2) + quiz | Shipped |
| Config, shell completions, line-based interactive menu (to be replaced by the ratatui TUI, B-017) | Shipped |
| CI pipeline (check / fmt / clippy / test) | Shipped |
| BEACON framework onboarding + state/architecture/product/standards/ops/compliance docs | Shipped — 2026-07-13 |
