# media-study - Development Tasks

## Project Vision
A CLI tool that ingests various media (PDFs, videos, notes, articles) into isolated "buckets" of knowledge, then uses LLMs to help with studying - answering questions grounded in source material, generating study guides, flashcards, homework help, and more.

**Key Principles:**
- Cross-platform (single binary for Windows/Mac/Linux)
- Open source, free (only cost is LLM API usage)
- Interactive CLI (guided prompts, not flag-heavy)
- Knowledge isolation via buckets (per class/project)
- Grounded responses (minimize hallucination, cite sources)

---

## Phase 1 - Foundation
> Basic CLI, Groq integration, file ingestion

- [x] Project scaffolding (Cargo, directory structure)
- [x] CLI with clap (subcommands: add, chat, config)
- [x] Interactive prompts with inquire (no flags needed)
- [x] Groq API client (chat completions)
- [x] Config management (~/.config/librarian/, migrated automatically from the legacy media-study directory)
- [x] PDF text extraction (pdf-extract)
- [x] Text/Markdown file reading
- [x] Basic chat loop (conversation history in memory)

---

## Phase 2 - Persistence
> Store ingested content, enable basic search

- [x] SQLite database setup (rusqlite, bundled)
- [x] Document storage schema (content, metadata, source path)
- [x] Persist ingested content to database
- [x] Full-text search (SQLite FTS5)
- [ ] Conversation history storage (deferred to Phase 3)
- [x] List ingested documents command
- [x] Delete/manage documents command

---

## Phase 3 - RAG & Buckets ✅
> Semantic search, knowledge isolation

- [x] Bucket management (create, list, switch, delete)
- [x] Per-bucket SQLite databases
- [x] Context retrieval for chat (FTS5 search)
- [x] Grounded responses with source citations
- [x] Embedding generation (local fastembed - all-MiniLM-L6-v2)
- [x] Vector storage (SQLite BLOBs with cosine similarity)
- [x] Semantic search within bucket
- [x] Chunking strategies (1000 char chunks with 200 char overlap)

---

## Phase 4 - Study Tools ✅
> Generate study materials from ingested content

- [x] Study guide generation (from bucket content)
- [x] Flashcard generation (Q&A pairs)
- [x] Quiz generation (multiple choice, fill-in-blank)
- [x] Homework help mode (guided problem solving)
- [x] Summary generation (per document or topic)
- [ ] Export to Markdown/PDF (save option added, full PDF export deferred)

---

## Phase 5 - Media Expansion ✅
> Video, audio, images, URLs

- [x] Audio transcription (Groq Whisper API)
- [x] Video transcription (extract audio → Whisper via ffmpeg)
- [x] URL/article ingestion (fetch and parse with scraper)
- [x] Image OCR (tesseract CLI)
- [x] YouTube URL support (yt-dlp transcript extraction)

---

## Phase 6 - Polish (In Progress)
> UX improvements, distribution

- [x] ASCII art banner and styled CLI interface
- [x] Comprehensive README with examples
- [x] LICENSE file (MIT)
- [x] Multi-agent code review completed:
  - CLI UX analysis (30 issues identified)
  - Code correctness review (17 issues identified)
  - Concurrency review (10 issues identified)
  - Rust best practices (25+ issues identified)
  - Documentation review (comprehensive)
  - Security audit (9 vulnerabilities identified)
- [x] Fix critical security issues:
  - [x] API key file permissions (0o600 on Unix)
  - [x] Command injection prevention (path validation for ffmpeg/tesseract/yt-dlp)
  - [x] SSRF protection (URL scheme validation, private IP blocking)
- [x] Fix blocking I/O in async contexts (tokio::fs)
- [x] Fix unsafe unwrap() calls that could panic
- [x] Unique temp file names to prevent conflicts
- [x] Progress bars for long operations (indicatif spinners and progress bars)
- [x] Streaming responses (real-time LLM output in chat and generate)
- [x] Shell completions (bash, zsh, fish, powershell via clap_complete)
- [x] GitHub Actions for CI and cross-platform releases
- [ ] Better error messages and recovery
- [ ] Homebrew formula / AUR package

---

## Next up — Phase 1: Quality (BEACON, 2026-07-13)
> Immediate, actionable steps. Tied to `docs/planning/BACKLOG.md` and `EXECUTION_PLAN.md`.

**B-001 — Promote chunk keyword arm to FTS5 (MEDIUM)**
- [ ] Add an FTS5 virtual table (`chunks_fts`) mirroring `chunks.content` (document-level search already uses `documents_fts`); keep it in sync on insert/delete (triggers or explicit writes in `src/storage/chunks.rs`)
- [ ] Rewrite `search_content` to query FTS5 with `MATCH` instead of `content LIKE ?`
- [ ] Handle existing local DBs: build the FTS index for already-ingested chunks (one-time backfill)

**B-002 — Expand automated test coverage (HIGH)**
> Baseline: 15 tests already cover pure functions in `search.rs`, `ingest/chunker.rs`, `ingest/ocr.rs`, `ingest/url.rs`, and `storage/study.rs` (SM-2 scheduler). Untested: ingest I/O, storage CRUD, embeddings, LLM/Whisper clients, command handlers.
- [ ] storage/chunks: insert, `get_all_with_embeddings`, `search_content`, delete/count (in-memory SQLite)
- [ ] embeddings: `embedding_to_bytes` / `bytes_to_embedding` round-trip; vector dimension = 384
- [ ] retrieval: hybrid scoring, Jaccard dedup, query enhancement (extend `src/search.rs` tests)
- [ ] ingest: chunking boundaries (1000-char chunk, 200-char overlap) and a small txt/md fixture

**B-003 — Fix the release pipeline (HIGH)**
- [ ] In `.github/workflows/release.yml`, replace `media-study` / `media-study.exe` with `librarian` / `librarian.exe` in the package/tar/7z/upload steps
- [ ] Rename produced archives (e.g. `librarian-<name>.tar.gz` / `.zip`)
- [ ] Dry-run via `workflow_dispatch` and confirm archives contain the real binary

**B-004 — Add `cargo audit` to CI**
- [ ] Add a CI job that runs `cargo audit` (install `cargo-audit` or use an action)
- [ ] Triage any advisories; decide fail-vs-warn policy for the pipeline

**B-009 — Keep docs in sync**
- [ ] Re-check README feature/model-routing claims against code as part of DoD for the above

---

## Backlog / Ideas
> Future possibilities, not committed

- [ ] Local LLM support (Ollama integration)
- [ ] Web UI option (optional local server)
- [ ] Spaced repetition for flashcards
- [ ] Collaborative buckets (share with classmates)
- [ ] Plugin system for custom ingestors
- [ ] VS Code extension
- [ ] Mobile companion app

---

## Notes

**Model routing strategy:**
| Task | Model | Reason |
|------|-------|--------|
| Quick Q&A | llama-3.1-8b-instant | Fast |
| Deep analysis | openai/gpt-oss-120b | Quality |
| Study guide | openai/gpt-oss-120b | Quality |
| Summarization | llama-3.1-8b-instant | Good enough |
| Transcription | whisper-large-v3-turbo | Groq Whisper (default; whisper-large-v3 for most accurate) |

**Grounding prompt strategy:**
- System prompt enforces source-only answers
- Include retrieved chunks in context
- Ask model to cite which document/section
- "I don't know" if not in materials
