---
title: "API CONTRACT"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: ApiContract
author: Derek Martinez
---

# API Contract — The Librarian

The Librarian is a **local CLI application**, not a web service. It exposes no
HTTP server and no public library API. Its "contract" has two parts:

1. **The CLI surface** it presents to users (subcommands, arguments, flags) — the
   user-facing interface that must stay backward-compatible.
2. **The external HTTP API it consumes** — the Groq cloud endpoints for chat and
   transcription.

## Part 1 — CLI Contract (user-facing)

Binary: `librarian`. Defined with `clap` (derive) in `src/main.rs`. Running with
no subcommand launches the interactive menu. `--version` and `--help` are provided
by clap. Every positional argument is **optional**; when omitted, the command
falls back to an interactive prompt (`inquire`).

| Command | Args | Description |
|---------|------|-------------|
| `add [PATH]` | `PATH`: file, directory, or URL (optional) | Ingest knowledge (PDF/text/audio/video/image/URL/YouTube) |
| `chat` | — | Interactive Q&A grounded in ingested materials |
| `list` | — | List documents in the current bucket |
| `search [QUERY]` | `QUERY`: search string (optional) | Semantic + full-text search over materials |
| `docs` | — | Manage documents (view / delete) |
| `delete [ID]` | `ID`: document id (`i64`, optional) | Remove a document |
| `bucket` (alias `library`) | subcommand (see below) | Manage libraries/buckets |
| `config` | — | Configure API key and model preferences |
| `generate` | subcommand (see below) | Generate study materials |
| `review` | — | Spaced-repetition study session |
| `quiz` | — | Interactive knowledge quiz |
| `completions <SHELL>` | `SHELL`: bash \| zsh \| fish \| powershell \| elvish | Emit shell completion script (required arg) |

### `bucket` / `library` subcommands

| Subcommand | Args | Description |
|------------|------|-------------|
| `bucket create [NAME]` | `NAME` (optional) | Create a new bucket |
| `bucket list` | — | List all buckets |
| `bucket use [NAME]` | `NAME` (optional) | Switch active bucket |
| `bucket delete [NAME]` | `NAME` (optional) | Delete a bucket (interactive) |
| `bucket` (no subcommand) | — | Interactive bucket management |

### `generate` subcommands

| Subcommand | Args | Description |
|------------|------|-------------|
| `generate study-guide [TOPIC]` | `TOPIC` (optional) | Comprehensive study guide |
| `generate flashcards [TOPIC]` | `TOPIC` (optional) | Flashcards for review |
| `generate quiz [TOPIC]` | `TOPIC` (optional) | Practice quiz |
| `generate summary [TOPIC]` | `TOPIC` (optional) | Summary of materials |
| `generate homework` | — | Interactive homework-help mode |
| `generate` (no subcommand) | — | Interactive generation menu |

### CLI stability expectations

- The subcommand names, their arguments, and the config file schema are the
  **public contract**. Changes should preserve backward compatibility.
- Renaming/removing a subcommand or flag, or changing an argument's meaning, is a
  **breaking change** and should bump the crate version (currently `0.1.0`) and be
  noted in `README.md` and the state docs.
- Additive changes (new optional subcommands/args) are non-breaking.

## Part 2 — Consumed External HTTP API (Groq)

The Librarian is a client of the Groq cloud API. It sends only LLM queries;
embeddings are computed locally (privacy-first). Endpoints (see `src/llm/`):

| Purpose | Endpoint | Source |
|---------|----------|--------|
| Chat / generation | `POST https://api.groq.com/openai/v1/chat/completions` | `src/llm/groq.rs` |
| Audio transcription | `POST https://api.groq.com/openai/v1/audio/transcriptions` | `src/llm/whisper.rs` |

- **Protocol:** OpenAI-compatible JSON API over HTTPS via `reqwest`. Transcription
  uses `multipart/form-data` for the audio upload.
- **Authentication:** Bearer token — `Authorization: Bearer <GROQ_API_KEY>`. The
  key is resolved from `config.toml` or the `GROQ_API_KEY` environment variable;
  it is never committed.
- **Models:**
  - Chat/generation: `llama-3.3-70b-versatile` (default, high quality) or
    `llama-3.1-8b-instant` (faster, lower latency).
  - Transcription: `whisper-large-v3-turbo` (default) or `whisper-large-v3` (most
    accurate) for audio/video.
- **Error handling:** responses are wrapped with `anyhow` context; failures
  (missing key, network error, non-2xx) surface as contextual user messages, not
  panics. Very large audio uploads may hit Groq size/time limits — split long
  recordings (see README troubleshooting).

### Local storage "contract"

Not a network API, but a stable on-disk shape callers depend on: one bundled
**SQLite** database per bucket (`rusqlite`, `bundled`), with documents, chunks +
384-dim embeddings, and FTS5 full-text search. Schema changes must remain
readable for existing user databases or provide a migration.
</content>
