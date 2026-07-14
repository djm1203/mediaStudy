---
title: "GLOSSARY"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Glossary
author: Derek Martinez
---

# Glossary — The Librarian

| Term | Definition |
|------|------------|
| The Librarian | The application itself: a local-first Rust CLI (binary `librarian`) that ingests a student's course materials and answers questions grounded in them. Chat mode is styled as "Ask the Librarian." |
| Bucket / Book | An isolated named collection of materials, one per class or project (e.g., `PSC-4395`). Each bucket is its own SQLite database with its own `generated/` directory. Managed via `librarian bucket` (alias `library`). |
| Document | A single ingested source (a PDF, note, transcript, OCR'd image, web article, etc.) stored in a bucket. Listed with `librarian list`/`docs` and removable by ID with `librarian delete`. |
| Chunk | A ~1000-character slice of a document's extracted text (with ~200-character overlap between adjacent chunks) that serves as the unit of embedding and retrieval. |
| Embedding | A 384-dimensional numeric vector representing a chunk's meaning, generated locally with the all-MiniLM-L6-v2 model so similar text lands near it in vector space. |
| FastEmbed | The Rust library used to run the local embedding model (all-MiniLM-L6-v2); the ~90MB model downloads once on first run. No text leaves the device to embed. |
| Semantic search | Retrieval by meaning: the query is embedded and compared to chunk embeddings by cosine similarity to find conceptually relevant passages, even without exact word matches. |
| Hybrid search | The Librarian's retrieval strategy that combines semantic (embedding cosine similarity) results with keyword (SQL LIKE) matching, so both conceptual matches and exact terms/references are surfaced. |
| Query enhancement | Pre-processing of a raw query before search: stripping filler phrases (e.g., "can you explain…") and extracting specific references (chapter, exercise, page, section, problem numbers) so keyword matching finds them. |
| Chunk deduplication | Removing near-duplicate retrieved chunks (>80% word overlap, Jaccard similarity), keeping the first occurrence, so context isn't padded with repeats. |
| Grounding / RAG | Retrieval-Augmented Generation: relevant chunks are retrieved from the bucket and passed as context to the LLM so answers are based on the student's own materials rather than the model's open-domain knowledge. |
| Groq | The external API provider used for LLM inference and transcription. Chosen for fast, free/low-cost inference. LLM prompts and transcription audio are the only data sent off-device. |
| Whisper transcription | Speech-to-text for audio/video, performed by Groq's whisper-large-v3-turbo model (default; whisper-large-v3 is a more-accurate alternative). FFmpeg decodes the media locally; the extracted audio is sent to Groq and the returned transcript is ingested. |
| OCR | Optical Character Recognition: local text extraction from images/screenshots (png/jpg/gif/bmp/tiff) via Tesseract, so figures and photographed notes become searchable text. |
| Spaced repetition | A review method that schedules items for recall at increasing intervals. `librarian review` steps through items that are due, reinforcing material over time. |
| Study guide | A generated, source-cited Markdown document of key concepts, details, relationships, and review points, produced by `librarian generate study-guide` and saved to the bucket's `generated/` directory. |
| Flashcards | Generated question/answer pairs for review, produced by `librarian generate flashcards`; also seed items for spaced-repetition review. |
| Quiz | A set of practice questions. `librarian generate quiz` produces a saved practice quiz; `librarian quiz` runs an interactive active-recall quiz (multiple-choice, fill-in-the-blank, short-answer). |
| Summary | A generated condensed overview of the materials or a specified topic/document, produced by `librarian generate summary`. |
| Homework help | An interactive, guided problem-solving mode (`librarian generate homework`) that walks the student through a solution using ingested course materials as context, rather than only returning a final answer. |
| Interactive TUI | The terminal interface shown when `librarian` runs with no arguments: banner, library shelf (buckets as books), status dashboard (current book, document/chunk counts, API-key status), and inquire-based menus. |
| Groq API key | The credential required for chat, generation, and transcription. Set via `librarian config` or the `GROQ_API_KEY` environment variable. |
