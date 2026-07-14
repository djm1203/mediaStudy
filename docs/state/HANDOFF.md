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

Populated the BEACON state and architecture/governance docs to reflect The Librarian's actual
architecture and current state (replacing the generic onboarding placeholders) and fact-checked them
against source. Then assessed the project and defined the production-readiness roadmap: 9 epics with
v1.0 = E1/E2/E3/E6/E7/E9 (see `docs/planning/BACKLOG.md` + `EXECUTION_PLAN.md`, decisions D-7/D-8/D-9).
Product direction is fixed: **local-first CLI, pluggable LLM providers, full-screen ratatui TUI.**

## What's Next — v1.0 build (autonomous, parallel agents)

Order: foundation first, then **E3 → E2 → E1**, with E6 (tests) continuous.

1. **Foundation:** E9/B-024 dependency modernization (`cargo upgrade`, verify build/tests), E7/B-003
   fix the release pipeline binary name (`librarian`, not `media-study`), E6/B-004 add `cargo audit` to CI.
2. **E3 — full-screen ratatui TUI (first feature epic, B-017/B-018):** add `ratatui`+`crossterm`,
   extract a new `src/tui/` module from `main.rs`, build the multi-pane app (library sidebar / content
   pane / status bar), streaming chat view, live search, ingestion dashboard, interactive study modes,
   theme + keybinding system. Keep `librarian <cmd>` subcommands intact for scripting.
3. **E2 — pluggable providers (B-015/B-005/B-016):** `LlmProvider`/`Transcriber` traits, Groq/OpenAI/
   Anthropic/Ollama adapters, retry/backoff, streaming.
4. **E1 — RAG quality (B-011/B-012/B-001/B-007/B-013/B-014):** structured per-chunk citations, hybrid
   rerank, `chunks_fts`, vector index, structure-aware chunking, retrieval eval harness.
5. **E6 — testing (B-002/B-009):** fill test gaps as each epic lands; keep docs in sync.

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

Runtime requirements: `GROQ_API_KEY` env var or `groq_api_key` in `config.toml` (OS data dir,
app name "librarian", migrated automatically from the legacy "media-study" directory). Optional binaries: FFmpeg (audio/video), Tesseract (OCR). First run
downloads the ~90MB embedding model.

## What to Watch

- Retrieval uses brute-force cosine over all chunk embeddings (no ANN index) — fine now, watch at large scale.
- The chunk-level keyword arm is `LIKE`-based rather than a dedicated chunk FTS index — a relevance/perf ceiling on that arm only (document-level search already uses FTS5).
- Groq API is a hard dependency for chat/generation and audio transcription; prompts and audio leave the machine.
- Ingestion feature degradation is silent when FFmpeg/Tesseract are absent.
