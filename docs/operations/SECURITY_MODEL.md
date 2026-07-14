---
title: "SECURITY MODEL"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: SecurityModel
author: Derek Martinez
---

# Security Model — The Librarian

The Librarian is a **local, single-user command-line tool** (`the-librarian` crate, `librarian` binary). It runs entirely on the user's own machine. There is no server, no hosted backend, no user accounts, and no multi-tenant surface. This document describes trust boundaries, assets, threats, and controls in terms proportionate to a local CLI — not a hosted service.

## Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│  User's machine (trusted)                                    │
│                                                              │
│   librarian CLI ── local SQLite DBs (per-bucket)            │
│        │          ── config.toml (Groq API key, plaintext)  │
│        │          ── fastembed ONNX model (local inference) │
│        │                                                     │
│        ├── subprocess: ffmpeg / tesseract (local files)      │
│        │                                                     │
└────────┼─────────────────────────────────────────────────────┘
         │  (network — data leaves the machine here)
         ▼
   ┌──────────────────────────┐   ┌──────────────────────────┐
   │ Groq API (3rd party)     │   │ Arbitrary web / YouTube  │
   │ - chat prompts + context │   │ (only when user ingests  │
   │ - audio for transcription│   │  a URL)                  │
   └──────────────────────────┘   └──────────────────────────┘
```

The only data that crosses the machine boundary is:

1. **Text sent to the Groq LLM API** — the system prompt, the user's question, and the retrieved context chunks selected by semantic search (`src/llm/groq.rs`, `https://api.groq.com/openai/v1/chat/completions`).
2. **Audio sent to the Groq Whisper API** — the extracted/normalized audio bytes from an audio or video file the user chose to transcribe (`src/llm/whisper.rs`, `https://api.groq.com/openai/v1/audio/transcriptions`).
3. **Outbound fetches the user explicitly requests** — when the user runs `librarian add <url>` or a YouTube URL, the tool fetches that remote page/video.

Everything else — ingestion, chunking, embedding (all-MiniLM-L6-v2 via fastembed), storage, and semantic search — happens locally with no network access.

## Assets

| Asset | Location | Sensitivity |
|-------|----------|-------------|
| User's study materials (PDFs, notes, audio, video, images, transcripts) | Local per-bucket SQLite DBs under the OS data dir | User-owned; may be copyrighted or personally sensitive |
| Generated study artifacts (guides, flashcards, quizzes, summaries) | `buckets/<name>/generated/` | Derived from user content |
| Chat / conversation history | Local SQLite | May contain sensitive study content |
| **Groq API key** | `config.toml` (plaintext) or `GROQ_API_KEY` env var | Secret — grants billable API access to the user's Groq account |
| Embedding model (ONNX, ~90MB) | `.fastembed_cache/` / OS cache dir | Downloaded artifact (supply-chain, see below) |

## Threats & Controls

### T1 — Groq API key leakage
The key is stored **in plaintext** in `config.toml`. This is a deliberate, documented trade-off for a local single-user tool (no OS keychain integration today).

- **Leak via config file exfiltration.** On Unix, `Config::save()` (`src/config.rs`) sets file permissions to `0o600` (owner read/write only). On Windows, no explicit ACL hardening is applied — the file inherits the user profile's default ACLs.
- **Leak via accidental commit.** `.gitignore` excludes `config.toml`, `*.db`, `*.db-*`, and `/data/`. The key lives in the OS config dir, outside the repo, so it is not in the working tree by default.
- **Leak via logs.** The key is only ever placed in the `Authorization: Bearer` header (`groq.rs`, `whisper.rs`). It is never written to stdout/stderr or debug logs. Error messages surface HTTP status + response body from Groq, not the request headers.
- **Env-var alternative.** `GROQ_API_KEY` is supported as a fallback (`Config::get_api_key`), letting security-conscious users keep the key out of any file entirely.

### T2 — Untrusted ingested content
The user ingests arbitrary files and URLs; a malformed or malicious PDF/page could crash a parser.

- **PDF parsing is hardened against panics** (`src/ingest/pdf.rs`): the primary extractor (`pdf-extract`) is wrapped in `panic::catch_unwind`, and on error *or* panic the code falls back to `lopdf`. Empty extractions produce a clean `anyhow` error rather than a crash.
- **All ingestion returns `Result`** and errors propagate as user-visible `anyhow` context messages, not process aborts.
- Ingested content is treated as **data, never executed**. It is chunked, embedded, and stored.
- **Note (residual risk):** retrieved chunks are later inserted into LLM prompts. Malicious ingested text could attempt prompt injection against the Groq model. Because output is advisory study text shown to the same user who ingested the content, the blast radius is limited, but users should be aware that ingesting untrusted material can influence generated answers.

### T3 — Subprocess handling (FFmpeg / Tesseract)
Video/audio and image ingestion shell out to external binaries.

- Paths passed to `ffmpeg` are **validated and canonicalized** first (`validate_path` in `whisper.rs`): existence check, regular-file check, and UTF-8 check, reducing path-traversal / special-file risks.
- Arguments are passed as an **explicit argv array** (`Command::new("ffmpeg").args([...])`), not a shell string, so there is no shell-injection surface.
- Temp output files are written to the OS temp dir with a PID+timestamp name to avoid collisions.
- These binaries are the user's own installed tools; their trust is inherited from the user's environment.

### T4 — Data exposure to a third party (Groq) — inherent, documented
When the user chats, generates, or transcribes, the relevant prompt text, retrieved context, or audio is sent to Groq. This is **inherent to the feature**, not a defect. Users must treat Groq as a processor of whatever they choose to query or transcribe. Purely local features (ingest, embed, semantic search, listing, deletion) never contact Groq. See `docs/compliance/DATA_GOVERNANCE.md`.

### T5 — Local file access scope
The tool reads files the user points it at and writes only within the OS config dir and OS data dir (`Config::config_dir`/`data_dir`, with a legacy `media-study` → `librarian` migration). It does not scan the filesystem, elevate privileges, or run as a service.

## Controls Summary

- Local-only embeddings and search (no network for the core RAG loop).
- No telemetry, analytics, or phone-home of any kind.
- Secrets never logged; env-var injection supported; `0o600` perms on Unix config.
- Panic-safe, fallback-based PDF parsing; `Result`-based error handling throughout.
- Argv-array subprocess invocation with path validation.
- Per-bucket SQLite isolation limits corruption/exposure blast radius.

## Explicitly Out of Scope

Because this is a single-user, local tool, the following are **not** applicable and are intentionally absent:

- **Authentication / authorization / user accounts** — the OS user is the sole trust principal; OS file permissions are the access-control boundary.
- **Multi-tenancy / tenant isolation** — there is one user and one machine.
- **Server-side hardening, rate limiting, WAF, network policy** — there is no server.
- **PII collection / user tracking** — none is collected; the only "personal data" is the user's own local files.
- **Encryption at rest of the local DBs** — relies on the user's own OS/disk encryption; not implemented in-app.
- **API-key encryption / OS keychain storage** — a known accepted limitation (plaintext config, mitigated by file perms and env-var option).
