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
