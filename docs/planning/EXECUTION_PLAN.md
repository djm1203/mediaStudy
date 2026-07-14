---
title: "EXECUTION PLAN"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: ExecutionPlan
author: Derek Martinez
---

# Execution Plan — the-librarian

Sequenced, realistic plan for a solo project run with autonomous parallel agents. Each item ties
to a backlog ID in `docs/planning/BACKLOG.md`. This is a **sequence, not a schedule** — no fixed
dates. Product direction (local-first CLI, pluggable providers, ratatui TUI) is fixed in
`docs/state/DECISIONS.md` (D-7/D-8/D-9).

## Phase 0 — Foundation (DONE, predates BEACON)

Core local-first RAG pipeline built and working: multi-format ingestion, local fastembed
embeddings, hybrid retrieval, per-bucket SQLite (with `documents_fts` FTS5), chat, study-material
generation, SM-2 spaced-repetition review, config, shell completions, line-based menu. CI
(check/fmt/clippy/test) is green. BEACON onboarded and the full doc set populated this session.

## Phase 1 — v1.0 "production-ready"

Six epics (E1, E2, E3, E6, E7, E9). Delivery order below; **E6 runs continuously** (every new
module ships with tests), and **E9 + E7/B-003 go first** as a clean foundation.

### Step A — Foundation (do first)
- **B-024 (E9)** Modernize dependencies (`cargo upgrade`; verify build/tests after each bump) so
  the TUI and provider work is built on current crates.
- **B-003 (E7)** Fix the broken release pipeline (binary name `librarian`).
- **B-004 (E6)** Add `cargo audit` to CI.

### Step B — E3, full-screen TUI (first feature epic)
Chosen to go first: it reshapes the app's structure early (extract `src/tui/` from `main.rs`),
gives the most visible payoff, and creates the surface the later epics render into.
- **B-017** ratatui multi-pane app (library sidebar / content pane / status bar), streaming chat
  view, live search, ingestion dashboard, interactive study modes. Keep CLI subcommands intact.
- **B-018** Theme + input system (light/dark + accent, vim/arrow keys, mouse, help overlay).

### Step C — E2, pluggable providers
Do before E1 so retrieval/citation work targets a stable provider seam.
- **B-015** `LlmProvider` + `Transcriber` traits; Groq/OpenAI/Anthropic/Ollama adapters; offline mode.
- **B-005** Retry/backoff, rate-limit + timeout handling, clear errors.
- **B-016** Streaming responses through the trait into the TUI chat view.

### Step D — E1, RAG quality & trust
- **B-011** Structured, verifiable citations (doc + chunk) with a browsable sources view in the TUI.
- **B-012** Hybrid reranking (RRF / weighted fusion) replacing concatenation.
- **B-001** Promote chunk keyword arm to a `chunks_fts` FTS5 table.
- **B-007** Vector index (evaluate `sqlite-vec`/`usearch`/HNSW) to replace brute-force cosine.
- **B-013** Structure-aware chunking with tunable overlap.
- **B-014** Retrieval eval harness wired into CI.

### Step E — E6/E7 close-out for v1.0
- **B-002** Fill remaining test gaps (ingest I/O, storage CRUD, embeddings, provider adapters, commands).
- **B-006 / B-023** Cross-platform prebuilt binaries on GitHub releases; crates.io publish + self-update.
- **B-009** README/docs synced with the shipped TUI + provider features.

**v1.0 exit criteria:** one-step install produces a working binary per platform; the full-screen
TUI is the default experience; chat/transcription work across at least Groq + one other provider
(incl. an offline Ollama path) with graceful retries; answers show verifiable per-chunk citations;
retrieval stays fast on a realistic multi-thousand-chunk library; the retrieval eval harness and
`cargo audit` run in CI; core paths are tested.

## Phase 2 — v1.1 polish (E4, E5, E8)

- **E4 (B-008, B-019)** Ingestion robustness: scanned-PDF/OCR fallback, content-hash dedup,
  resumable/batch imports, long-audio chunking, size guards.
- **E5 (B-020, B-021)** Data safety: bucket export/import + backup, `librarian stats`, schema
  versioning/migrations, safe re-embed on model change.
- **E8 (B-022, B-010)** First-run wizard, `librarian doctor`, OS-keychain key storage, Homebrew/
  Scoop/AUR packaging.

**v1.1 exit criteria:** a user can back up/move/restore a library safely, messy real-world inputs
ingest reliably, and onboarding is a guided first-run experience.

## Timeline

Solo project, no fixed dates. Sequence only: finish Phase 1 (v1.0) before Phase 2. Within Phase 1,
Step A precedes the feature epics; E3 → E2 → E1 is the feature order; E6 is continuous.
