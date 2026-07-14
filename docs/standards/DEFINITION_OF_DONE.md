---
title: "DEFINITION OF DONE"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: DefinitionOfDone
author: Derek Martinez
---

# Definition of Done — The Librarian

The Librarian is a local-first Rust CLI. "Done" is defined against the same gates
CI enforces (`.github/workflows/ci.yml`) plus the project's state-doc discipline.
Run the checks below locally before considering any change complete.

## New Feature

- [ ] **Compiles:** `cargo check --all-features` is clean.
- [ ] **Formatted:** `cargo fmt --all -- --check` reports no diffs.
- [ ] **Lint-clean:** `cargo clippy --all-features -- -D warnings` passes with
      zero warnings (no crate-wide `#[allow]` to dodge a real lint).
- [ ] **Tests pass:** `cargo test --all-features` is green.
- [ ] **New logic is tested where practical** — pure functions especially
      (search/dedup in `src/search.rs`, chunking in `ingest/chunker.rs`, OCR/URL
      parsing in `ingest/ocr.rs`/`ingest/url.rs`, the SM-2 scheduler in
      `storage/study.rs`). If the
      code is network/filesystem/external-process bound and cannot reasonably be
      unit-tested yet, say so explicitly rather than skipping silently.
- [ ] **No secrets committed.** The Groq API key is provided only via
      `librarian config` (`config.toml`) or the `GROQ_API_KEY` env var — never
      hardcoded or checked in.
- [ ] **Errors are contextual, not panics.** Fallible paths return
      `anyhow::Result` with `.context(...)`; no `unwrap()`/`expect()` on runtime
      I/O, network, or parsing. Interactive flows back out gracefully on Esc/Ctrl-C.
- [ ] **Cross-platform paths respected.** Use `dirs` for config/data locations;
      never hardcode `/home/...`, `~`, or `\` separators. Must work on Windows,
      macOS, and Linux.
- [ ] **User-facing surface documented if changed.** Update `README.md` and
      `docs/standards/API_CONTRACT.md` when CLI subcommands, args, or supported
      formats change; keep clap help text accurate.
- [ ] **State docs updated:** `docs/state/STATUS.md`, `HANDOFF.md`, `CHANGELOG.md`,
      and (as relevant) `DECISIONS.md`, `OPEN_QUESTIONS.md`, `RISKS.md`.

## CLI / UX Change

- [ ] New or changed subcommands/flags preserve backward compatibility, or the
      crate version is bumped to signal a breaking change (see `API_CONTRACT.md`).
- [ ] Shell completions still generate correctly (`librarian completions <shell>`).
- [ ] Interactive menu and non-interactive command paths both exercised for the
      changed behavior.

## Bug Fix

- [ ] Root cause identified and noted in the commit / state docs.
- [ ] Regression covered by a test where the fix touches testable logic.
- [ ] All existing tests pass; no unrelated changes bundled in.

## Refactor

- [ ] No behavior change (verified by existing tests / manual run).
- [ ] Kept separate from behavior-changing commits.
- [ ] `fmt`, `clippy -D warnings`, and `test` all pass before and after.

## Ingestion / External-Tool Change

- [ ] Degrades gracefully when an optional dependency (FFmpeg, Tesseract) or the
      Groq API key is missing — a clear, contextual message, not a panic.
- [ ] PDF path preserves the primary→`lopdf` fallback behavior.
- [ ] Any new supported format is reflected in `README.md` and the format table.

## Release Readiness

- [ ] `cargo build --release` produces a working `librarian` binary (LTO + strip).
- [ ] First-run embedding-model download still works; no unexpected new runtime
      prerequisites introduced without documenting them.
</content>
