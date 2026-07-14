---
title: "PRD"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Prd
author: Derek Martinez
---

# Product Requirements — The Librarian

## Overview

**The Librarian** is a personal, local-first AI study companion delivered as a
single Rust CLI (crate `the-librarian` v0.1.0, binary `librarian`). Students
ingest their own course materials — PDFs, lecture recordings, videos, notes,
screenshots, web articles, and YouTube lectures — into named **buckets** ("books",
one per class or project). They then chat with those materials, search across
them, and generate study aids (study guides, flashcards, quizzes, summaries,
homework help). Every answer is **grounded** in the student's ingested content via
retrieval-augmented generation (RAG).

Privacy is a first-class requirement: text extraction, chunking, embedding, and
storage all happen locally on the student's machine. The only data that leaves the
device is (a) LLM prompts sent to the Groq API for chat and generation, and
(b) audio extracted from media, sent to Groq's Whisper endpoint for transcription.
Groq is used because it offers fast and free/low-cost inference, keeping the tool
accessible to students.

## Problem Statement

Students accumulate scattered, multi-format course materials — PDF textbooks,
lecture recordings, slide screenshots, handwritten-to-photo notes, articles, and
online videos — with no single place to study from them. General-purpose chatbots
are not grounded in a student's actual materials (they hallucinate, cite nothing,
and miss course-specific content), and cloud study tools require uploading private
academic work to third-party servers. Students need trustworthy, source-grounded
study help that keeps their materials on their own machine and works across every
format their courses throw at them.

## Target Users

- **Students** (high school through university) organizing materials per class and
  preparing for exams, quizzes, and homework.
- **Self-learners** studying from mixed sources (textbooks, recorded talks,
  articles, video courses) who want a grounded study assistant over their own
  corpus.

Assumed comfort with a terminal; the tool ships a polished interactive TUI to lower
friction for non-expert CLI users.

## Technical Context

- **Primary language:** Rust (edition 2021), async via Tokio.
- **Build system:** Cargo. Release binary at `target/release/librarian`.
- **CLI/UX:** clap (subcommands + shell completions), inquire (interactive menus),
  colored / indicatif (styling and progress).
- **Storage:** one SQLite database per bucket (`documents.db`) with FTS5; generated
  study materials saved under each bucket's `generated/` directory.
- **Embeddings:** FastEmbed with all-MiniLM-L6-v2 (384-dim), generated locally.
- **LLM/transcription:** Groq API — whisper-large-v3-turbo (transcription default;
  whisper-large-v3 for most accurate),
  llama-3.3-70b-versatile (default chat/generation), llama-3.1-8b-instant (faster
  alternative).
- **External tools (optional):** FFmpeg (audio/video decode for transcription),
  Tesseract (image OCR).
- **Solution scope:** the entire repository; this is a single-solution project.

## Goals

- Frictionless multi-format ingestion (documents, audio, video, images, web,
  YouTube) into per-class buckets with a single `add` command.
- Fast, local semantic + keyword (hybrid) retrieval over ingested content.
- Grounded chat and study-material generation that draw only from the student's
  materials and cite sources where possible.
- Study-workflow support: generated guides/flashcards/quizzes, interactive quiz,
  and spaced-repetition review.
- Local-first privacy: no material stored or embedded off-device.
- Low cost / free to run for students (Groq free tier; local embeddings).

## Non-Goals

- **Not a cloud/SaaS service.** There is no hosted backend; everything runs on the
  student's machine (aside from Groq API calls).
- **Not multi-user.** No accounts, sharing, collaboration, or sync.
- **Not a general-purpose chatbot.** Responses are grounded in ingested materials,
  not open-domain knowledge.
- Not a document editor, note-taking app, or LMS integration.
- Not a provider-agnostic LLM layer — Groq is the assumed backend.

## Requirements

### Functional

- **R1 — Ingestion.** `librarian add <path|url|dir>` ingests PDF, txt, md; audio
  (mp3/wav/m4a/ogg/flac) and video (mp4/mkv/avi/mov/webm) via FFmpeg + Groq Whisper;
  images (png/jpg/jpeg/gif/bmp/tiff) via Tesseract OCR; web URLs; and YouTube
  videos. Directories are batch-imported. Extracted text is chunked (~1000 chars,
  200-char overlap) and embedded locally.
- **R2 — Buckets.** `librarian bucket create|list|use|delete` (alias `library`)
  manages isolated per-class books; each bucket is its own SQLite DB with its own
  `generated/` directory.
- **R3 — Search/retrieval.** `librarian search <query>`, `list`, `docs`,
  `delete <id>` provide hybrid retrieval: local embedding cosine similarity +
  keyword matching, with query enhancement (filler stripping, reference extraction)
  and chunk deduplication.
- **R4 — Chat.** `librarian chat` runs grounded interactive Q&A over the current
  bucket's materials.
- **R5 — Study-material generation.** `librarian generate <study-guide|flashcards|
  quiz|summary|homework>` produces materials grounded in the bucket, saved to the
  bucket's `generated/` directory (homework is interactive help).
- **R6 — Review & quiz.** `librarian review` runs a spaced-repetition session over
  due items; `librarian quiz` runs an interactive active-recall quiz.
- **R7 — Config & UX.** `librarian config` sets the Groq API key and default model;
  `librarian completions <shell>` emits shell completions; running `librarian` with
  no arguments launches the interactive TUI (banner, library shelf, status
  dashboard, menu).

### Non-Functional

- **Privacy:** text extraction, chunking, embedding, and storage occur locally;
  only LLM prompts and transcription audio are sent to Groq.
- **Performance:** retrieval is local and fast; the ~90MB embedding model downloads
  once on first run.
- **Portability:** Windows, macOS, and Linux.
- **Cost:** runnable on Groq's free tier; local embeddings incur no per-query cost.
- **Resilience:** PDF extraction falls back to an alternate extractor (lopdf) when
  the primary parser fails; the interactive menu recovers gracefully from
  per-command errors.

### Constraints

- Requires a Groq API key (via `librarian config` or `GROQ_API_KEY`).
- Audio/video ingestion requires FFmpeg; image OCR requires Tesseract (both
  optional and only needed for those formats).
- Large media may hit Whisper upload/size limits; long recordings may need
  splitting.

## Success Criteria

- Answers in chat and generated materials are grounded in ingested content and cite
  sources where applicable (no unsupported/hallucinated claims about course
  material).
- A student can ingest a mixed-format set of materials (PDF + recording + article)
  into a bucket and immediately ask exam-prep questions answered from those sources.
- Retrieval over a bucket returns relevant chunks quickly and without duplicate
  near-identical passages.
- Ingestion is low-friction: single files, whole directories, and URLs all import
  through one `add` command.
- The tool runs end-to-end using only the Groq free tier plus local embeddings.
- Each bucket remains isolated: content and generated materials never leak between
  classes.

## Out of Scope

- Hosted/multi-user/collaborative features, accounts, and cross-device sync.
- Open-domain answering not grounded in ingested materials.
- Non-Groq LLM providers and cloud-based embedding.
- Editing or authoring source documents within the tool.
