---
title: "BUILD AND TEST"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: BuildAndTest
author: Derek Martinez
---

# Build & Test — The Librarian

`the-librarian` is a Rust CLI (crate `the-librarian` v0.1.0, binary `librarian`,
edition 2024, ~6,725 LOC). It is a local-first RAG study companion. Everything is
built and tested with Cargo.

## Prerequisites

- **Rust toolchain** — install via [rustup](https://rustup.rs/). CI uses the
  `stable` toolchain; edition 2024 requires a recent stable (1.85+).
- **Groq API key** — required for full runtime (chat, generation, Whisper
  transcription). Not needed to build or to run the format/lint/check jobs.
  Provide via `librarian config` (writes `config.toml`) or the `GROQ_API_KEY`
  environment variable.
- **Embedding model** — on first run the app downloads the local
  `all-MiniLM-L6-v2` model (~90 MB) used by `fastembed`. Build does not need it;
  runtime and any test that exercises embeddings does.
- **Optional external binaries** (runtime only, feature-dependent media):
  - **FFmpeg** — audio/video transcription.
  - **Tesseract** — image / screenshot OCR.
  SQLite is bundled (`rusqlite` `bundled` feature); no system database is needed.

## Build Commands

```bash
# Fast type-check without producing a binary (matches CI `check`)
cargo check --all-features

# Debug build
cargo build

# Optimized release binary -> ./target/release/librarian
# release profile enables lto = true and strip = true
cargo build --release

# Run in place (debug)
cargo run

# Run with debug logging
RUST_LOG=debug cargo run
```

## Test Commands

```bash
# Run the full test suite
cargo test

# Match CI exactly (all features enabled)
cargo test --all-features
```

## Lint & Format Commands

```bash
# Formatting check (CI gate — must be clean)
cargo fmt --all -- --check

# Auto-format locally
cargo fmt

# Clippy with warnings treated as errors (CI gate)
cargo clippy --all-features -- -D warnings
```

## Build System

- **Tool:** Cargo (Rust, edition 2024).
- **Manifests:** `Cargo.toml` (single crate, single binary target `librarian` at
  `src/main.rs`), `Cargo.lock`.
- **Release profile:** `[profile.release]` sets `lto = true` and `strip = true`
  for a smaller, fully optimized single binary.

## CI Pipeline

Defined in `.github/workflows/ci.yml`. Triggers on `push` and `pull_request` to
`main`. Four independent jobs, all on `ubuntu-latest` with the `dtolnay/rust-toolchain@stable`
toolchain (`CARGO_TERM_COLOR=always`):

| Job | Command | Purpose |
|-----|---------|---------|
| Check | `cargo check --all-features` | Compiles / type-checks |
| Rustfmt | `cargo fmt --all -- --check` | Formatting gate |
| Clippy | `cargo clippy --all-features -- -D warnings` | Lint gate (no warnings allowed) |
| Test | `cargo test --all-features` | Runs the test suite |

A separate `.github/workflows/release.yml` builds cross-platform binaries on tag
push — see `DEPLOYMENT.md`.

## Test Categories

| Category | Command | Notes |
|----------|---------|-------|
| Unit | `cargo test --all-features` | In-module `#[cfg(test)]` blocks |
| Integration | — | None currently |
| E2E | — | None; the CLI is exercised manually |

## Code Coverage

Coverage is **thin and honestly incomplete**. There is no coverage tool or
gate configured. `cargo test` runs 15 passing tests. Unit tests exist in a handful
of modules, concentrated on pure functions:

- `src/search.rs` — query enhancement, chunk overlap/dedup logic (the bulk of the tests).
- `src/ingest/chunker.rs` — text chunking.
- `src/ingest/url.rs` — URL/YouTube parsing helpers.
- `src/ingest/ocr.rs` — OCR text-cleaning helpers.
- `src/storage/study.rs` — the SM-2 spaced-repetition review scheduler.

Most modules — `ingest` (pdf/text), `storage` (db/documents/chunks), `llm`
(groq/whisper), `embeddings`, `bucket`, and the `commands/*` handlers — have **no
unit tests**. Much of this code performs network I/O (Groq API), filesystem, or
external-process work that is not currently mocked. New logic, especially pure
functions, should add tests where practical (see `DEFINITION_OF_DONE.md`).

## Build Artifacts

- Local: `target/debug/librarian` (debug), `target/release/librarian` (release).
- CI release: per-target archives (`.tar.gz` on Unix, `.zip` on Windows) attached
  to a GitHub Release — see `DEPLOYMENT.md`. No retention policy is configured
  beyond GitHub's defaults.
</content>
</invoke>
