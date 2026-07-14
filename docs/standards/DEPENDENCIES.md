---
title: "DEPENDENCIES"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Dependencies
author: Derek Martinez
---

# Dependencies — The Librarian

## Dependency Manifest

The single manifest is `Cargo.toml` (crate `the-librarian`, Rust edition 2024).
The dependency tree is fully resolved and pinned by `Cargo.lock`.

## Runtime Crates

| Crate | Version | Purpose | Why chosen |
|-------|---------|---------|------------|
| `clap` | 4 (derive) | CLI argument parsing / subcommands | De-facto standard Rust CLI framework; derive macros keep command definitions declarative. |
| `clap_complete` | 4 | Generate shell completion scripts | First-party companion to `clap`. |
| `inquire` | 0.7 | Interactive TUI prompts and menus | Powers the guided menu, bucket pickers, and quiz/review prompts. |
| `tokio` | 1 (full) | Async runtime | Needed for concurrent network I/O to Groq (chat streaming, transcription). |
| `reqwest` | 0.12 (json, multipart, stream) | HTTP client for the Groq REST API | `multipart` uploads audio to Whisper; `stream` enables token streaming for chat. |
| `futures-util` | 0.3 | Stream combinators | Consume the streamed chat response body. |
| `serde` / `serde_json` | 1 | Serialization | Config (TOML) and Groq JSON request/response bodies. |
| `pdf-extract` | 0.7 | Primary PDF text extraction | Handles most text-based PDFs directly. |
| `lopdf` | 0.34 | PDF fallback parser | Recovers text when `pdf-extract` fails on malformed PDFs. |
| `anyhow` | 1 | Application error handling | Ergonomic `Result` with context across the app. |
| `thiserror` | 1 | Typed error definitions | For library-style error enums. |
| `dirs` | 5 | OS config/data directory discovery | Cross-platform locations for `config.toml` and bucket databases. |
| `toml` | 0.8 | Config file format | Human-editable config serialized via serde. |
| `colored` | 2 | ANSI terminal colors | CLI output styling. |
| `indicatif` | 0.17 | Progress bars/spinners | Feedback during ingestion, embedding, and transcription. |
| `termimad` | 0.30 | Markdown rendering in the terminal | Renders grounded answers and study material. |
| `rusqlite` | 0.31 (bundled, modern_sqlite) | Embedded SQLite storage | `bundled` compiles SQLite in — **no system SQLite dependency**; `modern_sqlite` enables FTS5 and current pragmas. |
| `chrono` | 0.4 (serde) | Date/time | Timestamps and SM-2 spaced-repetition scheduling. |
| `fastembed` | 4 | Local ONNX text embeddings | Runs all-MiniLM-L6-v2 (384-dim) entirely on-device — the core privacy property (no embedding calls leave the machine). |
| `scraper` | 0.25.0 | HTML parsing | Extract main content from fetched web pages. |
| `html2text` | 0.16.7 | HTML → plain text | Convert scraped HTML into ingestible text. |
| `url` | 2.5.8 | URL parsing/validation | Validate and normalize web/YouTube sources. |

## External (Non-Cargo) Dependencies

These are not crates; they are runtime services or binaries the tool relies on:

| Dependency | Required? | Notes |
|------------|-----------|-------|
| **Groq API** (chat + Whisper) | **Required** | Chat completions and audio transcription. Needs a `GROQ_API_KEY` (config file or env var). This is the one hard external service dependency. |
| **fastembed model (~90 MB)** | First run | all-MiniLM-L6-v2 ONNX weights are downloaded on first embedding and cached locally thereafter. |
| **FFmpeg** | Optional | Required only to ingest **video** (extracts the audio track before Whisper). |
| **Tesseract** | Optional | Required only for **image OCR** ingestion. |

## Update & Audit Policy

- **Patch updates:** apply within ~1 week of release (`cargo update`), verify with `cargo test`.
- **Minor updates:** evaluate within ~2 weeks; adopt if the test suite passes.
- **Major updates:** schedule and test deliberately (breaking changes in `rusqlite`,
  `reqwest`, `fastembed`, and `inquire` have historically required code changes).
- **Security patches:** apply promptly; treat critical CVEs as urgent (within 24h).
- **Auditing:** run `cargo audit` (RustSec advisory DB) before releases and after
  dependency bumps. Consider `cargo deny` for license/advisory gating in CI.
- **Pinning:** keep `Cargo.lock` committed so builds are reproducible.

## New Dependency Review

Before adding a crate, check:

- **License** compatibility with the project's MIT license (see below).
- **Maintenance** signals (recent releases, open-issue health, download counts).
- **Security** (RustSec advisories) and transitive-dependency footprint.
- **Necessity** — prefer the standard library or an existing dependency when reasonable.

## License Compliance

- The crate is MIT-licensed; all dependencies must be license-compatible.
- Copyleft (GPL) dependencies require review before inclusion.
- The embedding model (all-MiniLM-L6-v2) and any external binaries (FFmpeg,
  Tesseract) carry their own licenses and are the user's responsibility to install.
