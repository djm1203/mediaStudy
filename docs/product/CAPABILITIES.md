---
title: "CAPABILITIES"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Capabilities
author: Derek Martinez
---

# Capabilities — The Librarian

Concrete capabilities of the `librarian` CLI, grouped by area and mapped to the
subcommands that implement them. Only shipped behavior is listed.

## Ingestion

| Capability | Description | Command |
|-----------|-------------|---------|
| Add documents | Ingest PDF, txt, and Markdown; text extracted, chunked (~1000 chars, 200 overlap), and embedded locally. | `librarian add <file>` |
| Add audio | Transcribe mp3/wav/m4a/ogg/flac via FFmpeg + Groq Whisper, then ingest the transcript. | `librarian add <audio>` |
| Add video | Extract audio from mp4/mkv/avi/mov/webm via FFmpeg, transcribe with Whisper, then ingest. | `librarian add <video>` |
| Add images (OCR) | Extract text from png/jpg/jpeg/gif/bmp/tiff via Tesseract OCR. | `librarian add <image>` |
| Add web article | Scrape and ingest text from any http(s) URL. | `librarian add <url>` |
| Add YouTube video | Ingest a YouTube video's content (transcript). | `librarian add <youtube-url>` |
| Batch import directory | Recursively ingest every supported file in a directory. | `librarian add <dir>` |

PDF extraction falls back to an alternate extractor (lopdf) when the primary parser
fails.

## Library / Bucket Management

| Capability | Description | Command |
|-----------|-------------|---------|
| Create bucket | Create a new isolated "book" (per class/project) with its own SQLite DB. | `librarian bucket create <name>` |
| List buckets | List all buckets in the library. | `librarian bucket list` |
| Switch bucket | Set the active bucket for subsequent commands. | `librarian bucket use <name>` |
| Delete bucket | Remove a bucket and its contents. | `librarian bucket delete <name>` |
| Library alias | `library` is an alias for the `bucket` command group. | `librarian library` |

## Search / Retrieval

| Capability | Description | Command |
|-----------|-------------|---------|
| Hybrid search | Retrieve relevant chunks combining local embedding cosine similarity with keyword (LIKE) matching. | `librarian search <query>` |
| Query enhancement | Strip filler phrases and extract specific references (chapter/exercise/page/section numbers) to sharpen retrieval. | (applied within search/chat) |
| Chunk deduplication | Drop near-duplicate chunks (>80% word overlap, Jaccard) from results. | (applied within search/chat) |
| Browse collection | List all documents in the current bucket. | `librarian list` |
| Manage documents | View and manage documents interactively. | `librarian docs` |
| Delete document | Remove a document by ID. | `librarian delete <id>` |

## Chat

| Capability | Description | Command |
|-----------|-------------|---------|
| Grounded Q&A | Interactive chat ("Ask the Librarian") that answers from the current bucket's materials via RAG using the Groq LLM. | `librarian chat` |

## Study-Material Generation

Generated artifacts are saved to the active bucket's `generated/` directory.

| Capability | Description | Command |
|-----------|-------------|---------|
| Study guide | Generate a comprehensive, source-cited study guide (optionally focused on a topic). | `librarian generate study-guide [topic]` |
| Flashcards | Generate Q/A flashcards for review. | `librarian generate flashcards [topic]` |
| Quiz (generated) | Generate a practice quiz. | `librarian generate quiz [topic]` |
| Summary | Generate a summary of the materials or a specific topic/document. | `librarian generate summary [topic]` |
| Homework help | Interactive, guided problem-solving mode grounded in course materials. | `librarian generate homework` |

## Review / Quiz

| Capability | Description | Command |
|-----------|-------------|---------|
| Spaced-repetition review | Review due study items (e.g., flashcards/quiz items) in a spaced-repetition session. | `librarian review` |
| Interactive quiz | Active-recall quiz with multiple-choice, fill-in-the-blank, and short-answer questions. | `librarian quiz` |

## Config / UX

| Capability | Description | Command |
|-----------|-------------|---------|
| Configuration | Set the Groq API key and default model (also honors `GROQ_API_KEY`). | `librarian config` |
| Shell completions | Emit tab-completion scripts for bash/zsh/fish/PowerShell/etc. | `librarian completions <shell>` |
| Interactive TUI | With no arguments, launch the interactive menu: banner, library shelf, status dashboard (current book, doc/chunk counts, API-key status), inquire-based menus. | `librarian` |

## Technical Capabilities

- **Primary language:** Rust (edition 2021), async (Tokio).
- **Frameworks/libraries:** clap + clap_complete, inquire, colored, indicatif,
  FastEmbed (all-MiniLM-L6-v2, 384-dim, local), SQLite (rusqlite) with FTS5,
  Groq HTTP API.
- **Storage model:** one SQLite database per bucket, plus a default DB when no
  bucket is selected; generated materials on disk per bucket.

## Integration Points

| Integration | Purpose | Data leaving device |
|-------------|---------|---------------------|
| Groq LLM API (llama-3.3-70b-versatile / llama-3.1-8b-instant) | Chat and study-material generation | LLM prompts + retrieved context |
| Groq Whisper API (whisper-large-v3-turbo default; whisper-large-v3 most accurate) | Audio/video transcription | Extracted audio only |
| FFmpeg (local, optional) | Decode audio/video for transcription | None (local) |
| Tesseract (local, optional) | Image/screenshot OCR | None (local) |
| FastEmbed / all-MiniLM-L6-v2 (local) | Embedding generation | None (local; ~90MB model downloaded once) |

## Capability Matrix

| ID | Capability | Status | Priority | Owner |
|----|-----------|--------|----------|-------|
| C-001 | Multi-format ingestion (`add`) | Shipped | HIGH | Derek Martinez |
| C-002 | Bucket/library management | Shipped | HIGH | Derek Martinez |
| C-003 | Hybrid semantic + keyword search | Shipped | HIGH | Derek Martinez |
| C-004 | Grounded chat | Shipped | HIGH | Derek Martinez |
| C-005 | Study-material generation | Shipped | HIGH | Derek Martinez |
| C-006 | Spaced-repetition review | Shipped | MEDIUM | Derek Martinez |
| C-007 | Interactive quiz | Shipped | MEDIUM | Derek Martinez |
| C-008 | Config, completions, interactive TUI | Shipped | MEDIUM | Derek Martinez |
