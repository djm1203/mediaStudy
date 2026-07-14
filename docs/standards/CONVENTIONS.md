---
title: "CONVENTIONS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Conventions
author: Derek Martinez
---

# Coding Conventions — The Librarian

## Language

Primary: **Rust**, edition **2024**. Async runtime is **Tokio** (`#[tokio::main]`,
`features = ["full"]`). The crate is a single binary (`librarian`).

## Formatting & Linting (enforced)

- **rustfmt** is the single source of truth for formatting. Run `cargo fmt`
  before committing. CI runs `cargo fmt --all -- --check` and fails on any diff.
- **Clippy runs clean with `-D warnings`.** No warnings are allowed to land —
  CI runs `cargo clippy --all-features -- -D warnings`. Fix lints rather than
  silencing them; when a suppression is genuinely warranted use a narrowly scoped
  `#[allow(...)]` (e.g. `#[allow(dead_code)]` on the currently-unused
  `print_header` in `main.rs`) rather than a crate-wide allow.
- No default rustfmt config overrides are checked in; the project uses stock
  rustfmt defaults.

## Naming

- Follow standard Rust conventions: `snake_case` for functions, variables,
  modules, and files; `CamelCase` for types and enum variants; `SCREAMING_SNAKE_CASE`
  for constants (e.g. `GROQ_API_URL`, `BANNER`).
- File names are `snake_case.rs` and match their module names.

## Module Organization

Modules are declared in `src/main.rs` and laid out as **folder-per-domain** with a
`mod.rs`-style entry per directory. Each domain owns a subfolder:

```
src/
├── main.rs        # CLI entry point (clap), interactive menu, banner/dashboard UI
├── config.rs      # Config load/save, API-key resolution
├── search.rs      # Query enhancement + chunk overlap/dedup (unit-tested)
├── render.rs      # Terminal/markdown rendering
├── bucket/        # Library "bucket" isolation (per-class/project DBs)
├── commands/      # One module per CLI subcommand handler
│   ├── add.rs  chat.rs  generate.rs  docs.rs  bucket.rs  config.rs  review.rs  quiz.rs
├── ingest/        # Media ingestion: pdf, text, url, ocr, chunker (chunker/ocr/url unit-tested)
├── llm/           # LLM clients: groq (chat), whisper (transcription)
├── embeddings/    # Local FastEmbed embedding generation
└── storage/       # SQLite layer: db, documents, chunks, study (study SM-2 scheduler unit-tested)
```

Keep the boundary clean: `commands/*` orchestrates user interaction and delegates
the actual work to `ingest`, `llm`, `embeddings`, `storage`, and `search`.

## Error Handling

- **`anyhow::Result<T>` is the default return type** throughout the application
  and command layer (including `fn main() -> anyhow::Result<()>`).
- **Attach context at every fallible boundary** with `.context("...")` /
  `.with_context(|| ...)` so user-facing errors are actionable rather than opaque.
- **Typed errors use `thiserror`** for domain error enums where a caller needs to
  match on the variant; these convert into the `anyhow` chain at the boundary.
- **Prefer contextual errors over panics.** Do not `unwrap()`/`expect()` on
  fallible runtime paths (I/O, network, parsing). The interactive loop in
  `main.rs` catches command errors and returns to the menu instead of crashing,
  and treats "cancelled"/"interrupted" as a normal back action.

## Interactive UX

- **`clap`** (derive API) defines the non-interactive CLI; **`clap_complete`**
  generates shell completions.
- **`inquire`** drives interactive prompts/menus (`Select`, etc.); handle
  `OperationCanceled` / `OperationInterrupted` gracefully (Esc / Ctrl-C should
  back out, not error).
- **`colored`** for terminal styling, **`indicatif`** for progress bars/spinners,
  **`termimad`** for rendering markdown in the terminal. Keep the polished,
  emoji-accented visual style (banner, library shelf, status dashboard) consistent.

## Documentation

- Use `///` doc comments on public items, commands, and non-obvious helpers
  (see the documented functions and enum variants in `main.rs`).
- Keep user-facing help text (clap `about` / `long_about`, subcommand doc
  comments) accurate — it is the CLI's public contract (see `API_CONTRACT.md`).

## Testing

- Unit tests live in-module under `#[cfg(test)] mod tests { ... }`, focused on
  pure functions (chunking, search/dedup, URL and OCR helpers, study storage).
- Add tests alongside new pure logic. Network/filesystem/external-process code is
  currently untested and not mocked — call this out honestly rather than claiming
  coverage that does not exist.

## Commit Policy (from CLAUDE.md, R-10.5)

- **Never commit unprompted** — create a commit only when the operator explicitly
  asks for one.
- **No AI attribution** — no `Co-Authored-By`, "Generated with", "written by", or
  any trailer/footer naming an AI agent, model, or tool.
- **Keep messages concise** — a short subject and at most a sentence or two of
  body describing what changed and why. Detailed history belongs in the
  `docs/state/*` files, not the commit message.
</content>
