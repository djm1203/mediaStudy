---
title: "DATA GOVERNANCE"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: DataGovernance
author: Derek Martinez
---

# Data Governance — The Librarian

The Librarian is a **local, single-user CLI**. All data lives on the user's own machine, and **the user is the data controller** of their own device. The tool does not collect data about the user, does not operate a backend, and gathers no third-party PII. The only data that ever leaves the machine is what the user actively sends to the Groq API by chatting, generating, or transcribing (see `docs/operations/SECURITY_MODEL.md`).

## Data Types & Classification

| Data type | Description | Classification | Location |
|-----------|-------------|----------------|----------|
| Study materials | User-ingested PDFs, text/Markdown, audio, video, images, web/YouTube content | User-owned; potentially sensitive or copyrighted | Local per-bucket SQLite (`buckets/<name>/documents.db`) as extracted text + chunks |
| Embeddings | 384-dim vectors (all-MiniLM-L6-v2) of chunked content | Derived from user content | Same per-bucket DB |
| Generated artifacts | Study guides, flashcards, quizzes, summaries | Derived; may echo source content | `buckets/<name>/generated/` |
| Chat / conversation history | User questions and model answers | May contain sensitive study content | Local SQLite |
| **Groq API key** | Credential for the user's Groq account | **Secret** | `config.toml` (plaintext, `0o600` on Unix) or `GROQ_API_KEY` env var |

## Where Data Is Stored

Everything is under OS-standard directories chosen by the `dirs` crate (`Config::config_dir`/`data_dir`), app folder `librarian` (with automatic migration from a legacy `media-study` folder):

- **Config dir** (`config.toml`): Linux `~/.config/librarian`, macOS `~/Library/Application Support/librarian`, Windows `%APPDATA%\librarian`.
- **Data dir**: Linux `~/.local/share/librarian`, macOS `~/Library/Application Support/librarian`, Windows `%APPDATA%\librarian`. Contains `default.db`, and `buckets/<name>/{documents.db, generated/}`.

Data is **isolated per bucket** ("book"), which localizes both corruption blast radius and any manual cleanup.

## Data Lifecycle

1. **Collection.** Only content the user explicitly ingests (`librarian add ...`). No background collection, scanning, or profiling.
2. **Processing (local).** Extraction (PDF/OCR/transcription-result), chunking (~1000 chars, 200 overlap), and embedding all happen locally.
3. **Processing (third party — only on user action).** Chat/generation send the prompt + retrieved context to Groq; transcription uploads extracted audio to Groq Whisper. See "Third-Party Processing" below.
4. **Storage.** Local SQLite, per bucket, plus generated files on disk.
5. **Retention.** **User-controlled — indefinite until the user deletes.** There is no automatic expiry.
6. **Deletion.** Deleting a document removes it and cascade-deletes its chunks/embeddings; deleting a bucket (`librarian bucket delete <name>`) removes that book's DB and generated artifacts. Removing the OS data dir removes everything.

## Third-Party Processing (Groq)

- **Chat / generation:** the system prompt, the user's question, and the retrieved context chunks are sent to `api.groq.com` (`src/llm/groq.rs`).
- **Transcription:** the extracted/normalized audio bytes from an audio or video file are uploaded to Groq Whisper (`src/llm/whisper.rs`).
- Users should treat anything they choose to chat about or transcribe as **disclosed to Groq**, and are responsible for reviewing Groq's applicable terms and privacy policy for how submitted data is handled. Purely local operations (ingest, embed, search, list, delete) never contact Groq.

## PII Handling

- The tool **collects no PII about the user** and performs no tracking or analytics.
- Any personal or sensitive information present is **content the user themselves ingested** (e.g. their own notes). The tool treats it as opaque user content, stores it only locally, and never logs it.
- The one secret handled — the Groq API key — is never logged and is used solely in the `Authorization` header.

## Access Control

- Access control is the **OS file-permission boundary**: whoever can read the user's config/data dirs can read the data. On Unix, `config.toml` is written `0o600`.
- There are no in-app accounts, roles, or service accounts — single user, single machine.
- Data-at-rest encryption is delegated to the user's OS/disk encryption; it is not performed by the app.

## Data Subject Rights (self-service)

Because the user controls their own machine, rights are exercised directly:

| Right | How |
|-------|-----|
| Access / export | Files are plain SQLite DBs and generated files in the data dir; the user can open/copy them directly |
| Rectification | Re-ingest corrected source material |
| Erasure ("right to delete") | Delete a document (cascades chunks), delete a bucket, or remove the OS data dir |
| Restrict third-party processing | Simply don't run chat/generate/transcription; use local search only |

## Retention Summary

| Data | Retention | Deletion method |
|------|-----------|-----------------|
| Study materials & embeddings | Until user deletes (no auto-expiry) | Delete document (cascade) / delete bucket |
| Generated artifacts | Until user deletes | Remove files in `buckets/<name>/generated/` |
| Chat history | Until user deletes | Delete via app / remove DB |
| Groq API key | Until user changes/removes it | Edit `config.toml` or unset env var; rotate at Groq console |
