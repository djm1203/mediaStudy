---
title: "ARCHITECTURE"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Architecture
author: Derek Martinez
---

# Architecture — The Librarian

**The Librarian** is a local-first, retrieval-augmented (RAG) "personal AI study
companion" delivered as a single-binary CLI. You ingest study materials (PDFs,
text/markdown, audio, video, images via OCR, web pages, and YouTube transcripts),
the tool chunks and embeds them **locally**, stores them per-"bucket" in SQLite,
and then lets you chat, quiz, and generate study materials grounded in your own
documents. The only data that leaves the machine is the text of your prompts (to
the Groq LLM) and audio files (to Groq Whisper for transcription); embeddings are
computed on-device.

- **Crate:** `the-librarian` v0.1.0 — **binary:** `librarian`
- **Language:** Rust (edition 2024)
- **Size:** ~6.7k LOC

## Language Breakdown

| Language | %   | Notes |
|----------|-----|-------|
| Rust     | ~100% | Edition 2024, ~6,725 LOC across `src/` |

## Build System

**Cargo** (manifest: `Cargo.toml`).

- Produces a single binary named `librarian` (`[[bin]]` → `src/main.rs`).
- Release profile enables `lto = true` and `strip = true` for a smaller, faster
  artifact.
- No external build steps: SQLite is compiled in via `rusqlite`'s `bundled`
  feature, so there is no system SQLite dependency.

```bash
cargo build --release      # optimized single binary at target/release/librarian
cargo run -- <subcommand>  # dev run
cargo test                 # unit tests (chunker, search, SM-2 spaced repetition)
```

## Frameworks / Key Libraries

| Concern | Library |
|---------|---------|
| CLI parsing + shell completions | `clap` (derive), `clap_complete` |
| Interactive TUI prompts / menus | `inquire` |
| Async runtime | `tokio` (full) |
| HTTP client (Groq REST) | `reqwest` (json, multipart, stream) + `futures-util` |
| Storage | `rusqlite` (bundled SQLite, `modern_sqlite`) |
| Local embeddings | `fastembed` (ONNX, all-MiniLM-L6-v2, 384-dim) |
| PDF extraction | `pdf-extract` (primary), `lopdf` (fallback) |
| Web ingestion | `scraper`, `html2text`, `url` |
| Terminal rendering | `termimad` (markdown), `colored`, `indicatif` |
| Config / paths | `dirs`, `toml`, `serde`/`serde_json` |
| Errors | `anyhow`, `thiserror` |
| Time | `chrono` |

See [DEPENDENCIES.md](../standards/DEPENDENCIES.md) for rationale per crate.

## Module Structure

```
src/
├── main.rs            # clap CLI + inquire interactive menu (entry point)
├── config.rs          # Config: groq_api_key / default_model / current_bucket;
│                      #   TOML at OS config dir; GROQ_API_KEY env fallback
├── render.rs          # termimad markdown rendering for the terminal
├── search.rs          # enhance_query (filler stripping + reference extraction),
│                      #   Jaccard chunk overlap + deduplicate_chunks
├── bucket/            # per-book isolation → buckets/<name>/{documents.db, generated/}
├── embeddings/        # fastembed model (loaded once), cosine_similarity,
│                      #   embedding_to_bytes / bytes_to_embedding (f32 LE BLOB)
├── ingest/            # source extraction + chunking
│   ├── chunker.rs     #   ~1000-char chunks, 200-char overlap, boundary-aware
│   ├── pdf.rs         #   pdf-extract with lopdf fallback
│   ├── text.rs        #   txt / markdown
│   ├── url.rs         #   scraper + html2text; YouTube transcript handling
│   └── ocr.rs         #   Tesseract (external binary) for images
├── llm/               # remote model clients
│   ├── groq.rs        #   Groq chat completions (default llama-3.3-70b-versatile)
│   └── whisper.rs     #   Groq Whisper transcription (whisper-large-v3[-turbo])
├── commands/          # one module per subcommand
│   ├── add.rs         #   ingest → chunk → embed → store
│   ├── chat.rs        #   hybrid retrieval + grounded generation
│   ├── quiz.rs        #   interactive quizzing
│   ├── generate.rs    #   generate flashcards / study material → generated/
│   ├── review.rs      #   spaced-repetition review (SM-2)
│   ├── docs.rs        #   list / inspect documents
│   ├── bucket.rs      #   create / switch / delete buckets
│   └── config.rs      #   API key + default model setup
└── storage/           # SQLite persistence (one connection per bucket DB)
    ├── db.rs          #   Database::open, init_schema (CREATE TABLE IF NOT EXISTS)
    ├── documents.rs   #   documents table + documents_fts (FTS5) search
    ├── chunks.rs      #   chunk content + embedding BLOB; keyword LIKE search
    ├── conversations.rs #  chat history (conversations + messages)
    └── study.rs       #   study_items + SM-2 spaced repetition
```

| Module | Responsibility |
|--------|----------------|
| `main` | Parse CLI (clap) or drive the interactive `inquire` menu; dispatch to commands. |
| `config` | Load/save `config.toml` (API key, default model, active bucket) in the OS config dir; `GROQ_API_KEY` env fallback; writes `0600` perms on Unix. |
| `bucket` | Isolate each subject/book as a directory `buckets/<name>/` with its own `documents.db` and `generated/` output folder. |
| `ingest` | Turn any supported source into plain text, then split into overlapping chunks. |
| `embeddings` | Lazily initialize the local ONNX model once (`OnceLock<Mutex<TextEmbedding>>`); embed text; cosine similarity; (de)serialize vectors to/from BLOBs. |
| `search` | Pre-process queries and de-duplicate near-identical retrieved chunks. |
| `llm` | Talk to Groq for chat completions (streamed) and Whisper transcription. |
| `storage` | Create schema and read/write documents, chunks, conversations, and study items. |
| `commands` | Implement each user-facing verb. |
| `render` | Pretty-print markdown responses in the terminal. |

## Data Flow

### Ingestion pipeline (`librarian add <source>`)

```
 source (pdf/txt/md/audio/video/image/url/youtube)
        │
        ▼
 ingest::extract_from_file_async / fetch_url
   ├─ pdf      → pdf-extract (fallback: lopdf)
   ├─ text/md  → read as UTF-8
   ├─ audio    → Groq Whisper API  ── network ──▶ transcript
   ├─ video    → FFmpeg extract audio ─▶ Whisper ─▶ transcript
   ├─ image    → Tesseract OCR (local) ─▶ text
   └─ url      → reqwest fetch → scraper/html2text (YouTube: transcript)
        │
        ▼  plain text
 ingest::chunk_text  (chunk_size≈1000 chars, overlap≈200, boundary-aware)
        │
        ▼  Vec<Chunk>
 embeddings::embed_texts  (LOCAL ONNX all-MiniLM-L6-v2 → 384-dim f32)
        │
        ▼
 storage: documents.insert(...)   → documents(+documents_fts trigger)
          chunks.insert(..., embedding_to_bytes(&emb))  → chunks(embedding BLOB)
        │
        ▼
 buckets/<active>/documents.db     (per-bucket isolation)
```

### Query / retrieval pipeline (`librarian chat`)

```
 user question
        │
        ▼
 search::enhance_query   (strip filler prefixes/suffixes; extract references
                          like "exercise 0.3", "page 26" as keyword terms)
        │
        ├──────────────── HYBRID RETRIEVAL ────────────────┐
        ▼                                                   ▼
 SEMANTIC (local)                                    KEYWORD (SQL)
 embeddings::embed_text(query)                       chunks.search_content(query)
 cosine vs. every chunk BLOB                          content LIKE '%kw%' (any term)
 → top 10 chunk ids                                   → top 10 chunk ids
        │                                                   │
        └───────────────► merge (keyword-first, then ◄──────┘
                          semantic; dedup by id)
        │
        ▼
 search::deduplicate_chunks  (drop chunks with >80% Jaccard word overlap)
        │
        ▼
 assemble CONTEXT block (dynamic char budget from model window),
 tag each with [Source: filename]
        │
        ▼
 llm::groq chat_stream  ── network ──▶  grounded answer (streamed)
        │
        ▼
 render (terminal) + persist to conversations/messages
```

If a bucket has documents but no chunk embeddings yet, retrieval falls back to
document-level **FTS5** search (`documents_fts`), then to raw document previews.

**Privacy boundary.** Extraction, chunking, embedding, storage, and similarity
scoring all happen on-device. Only two things cross the network: (1) prompt/context
text sent to the Groq chat API, and (2) audio uploaded to Groq Whisper for
transcription.

## Storage Schema

Each bucket owns an independent SQLite file (`buckets/<name>/documents.db`; the
default/no-bucket store is `default.db`). Schema is created idempotently via
`CREATE TABLE IF NOT EXISTS` in `storage/db.rs` and `storage/chunks.rs`.

| Table | Key columns | Notes |
|-------|-------------|-------|
| `documents` | `id` PK, `source_path`, `filename`, `content_type`, `content`, `tags?`, `created_at`, `updated_at` | Full extracted text of each source. |
| `documents_fts` | FTS5 virtual table over (`filename`, `content`, `tags`) | `content='documents'`, `content_rowid='id'`; kept in sync by AFTER INSERT/UPDATE/DELETE triggers. Used for document-level search / retrieval fallback. |
| `chunks` | `id` PK, `document_id` FK→`documents(id)` **ON DELETE CASCADE**, `chunk_index`, `content` TEXT, `embedding` BLOB (nullable) | Embedding is 384 × f32 little-endian bytes. Index: `idx_chunks_document_id`. Keyword search is `content LIKE '%kw%'`. |
| `conversations` | `id` PK, `title?`, `created_at`, `updated_at` | Chat sessions. |
| `messages` | `id` PK, `conversation_id` FK→`conversations(id)` **ON DELETE CASCADE**, `role`, `content`, `created_at` | Persisted chat history. |
| `study_items` | `id` PK, `document_id?` FK→`documents(id)` **ON DELETE SET NULL**, `item_type`, `front`, `back`, `next_review_date`, `interval_days`, `ease_factor`, `review_count`, `created_at`, `updated_at` | Flashcards/quiz items with SM-2 spaced-repetition scheduling. |

See [SCHEMA_CHANGE_POLICY.md](../standards/SCHEMA_CHANGE_POLICY.md) for how to
evolve this schema safely (especially the embedding/model coupling).

## External Integrations

| Integration | Required? | Used for |
|-------------|-----------|----------|
| **Groq chat completions** (`api.groq.com/openai/v1/chat/completions`) | Required | Grounded answers, quiz/study generation. Default model `llama-3.3-70b-versatile`. |
| **Groq Whisper** (`.../v1/audio/transcriptions`) | Required for audio/video | Transcription. Default `whisper-large-v3-turbo` (also `whisper-large-v3`). |
| **FFmpeg** (external binary) | Optional | Extract audio track from video before transcription. |
| **Tesseract** (external binary) | Optional | OCR text out of images. |
| **fastembed model download** | First-run only | ~90 MB all-MiniLM-L6-v2 ONNX model fetched on first embedding, then cached locally. |

Configuration lives at `<os-config-dir>/librarian/config.toml` (auto-migrated from
the legacy `media-study` directory) and holds the Groq API key, default model, and
active bucket. The key can also be supplied via the `GROQ_API_KEY` environment
variable.

## Known Gaps / Constraints

- **Keyword search is `LIKE`, not FTS5, at the chunk level.** The hybrid retriever's
  keyword arm (`chunks.search_content`) runs `content LIKE '%kw%'` over chunk text.
  FTS5 exists only for *document*-level search (`documents_fts`). (READMEs that
  describe chunk retrieval as FTS5 are inaccurate.)
- **Brute-force cosine similarity.** Semantic search loads every chunk's embedding
  and scores it linearly (O(n) per query) — there is no ANN index. This is fine for
  personal-scale libraries but sets a practical scaling ceiling (see
  [PERFORMANCE_BUDGET.md](../standards/PERFORMANCE_BUDGET.md)).
- **Embeddings are model-coupled.** Stored vectors are meaningful only for the
  384-dim all-MiniLM-L6-v2 model; switching embedding models invalidates every
  stored vector and requires a full re-embed.

## Decisions

See [DECISIONS.md](../state/DECISIONS.md).
