---
title: "USE CASES"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: UseCases
author: Derek Martinez
---

# Use Cases — The Librarian

## Actors

| Actor | Description |
|-------|-------------|
| Student | Primary user; organizes course materials into buckets, studies, and prepares for exams/homework. |
| Self-learner | Studies from mixed personal sources (textbooks, recorded talks, articles, videos). |
| The Librarian (system) | The CLI: ingests content, runs local retrieval, and calls Groq for grounded chat/generation. |

## Use Case Catalog

### UC-001: Build a class bucket and ask exam-prep questions

**Actor:** Student
**Preconditions:** Groq API key configured; FFmpeg installed (for the recordings).
**Flow:**
1. `librarian bucket create "PSC-4395"`
2. `librarian bucket use "PSC-4395"`
3. `librarian add ~/Documents/PSC/` (batch-imports PDFs and lecture recordings; recordings are transcribed via Whisper)
4. `librarian chat` — ask "What are the main causes discussed in week 3?"

**Postconditions:** Answers are grounded in the ingested PDFs and transcripts, with source references where available.
**Error flows:** Missing API key → run `librarian config`; missing FFmpeg → install it or add only documents.

### UC-002: Generate flashcards and a quiz from a textbook chapter

**Actor:** Student
**Preconditions:** Chapter PDF ingested into the active bucket.
**Flow:**
1. `librarian add chapter-3.pdf`
2. `librarian generate flashcards "Chapter 3"`
3. `librarian generate quiz "Chapter 3"`

**Postconditions:** Flashcards and a practice quiz are generated from the chapter and saved to the bucket's `generated/` directory.
**Error flows:** No relevant content found → broaden the topic or ingest more material.

### UC-003: Guided homework help grounded in course notes

**Actor:** Student
**Preconditions:** Course notes/materials ingested.
**Flow:**
1. `librarian generate homework`
2. Enter the problem; the Librarian walks through a guided, step-by-step solution grounded in the ingested notes rather than just giving the final answer.

**Postconditions:** Student is guided to the solution using their own course materials as context.
**Error flows:** Question references material not in the bucket → ingest the relevant source first.

### UC-004: Summarize a long lecture video transcript

**Actor:** Student
**Preconditions:** FFmpeg installed; Groq API key configured.
**Flow:**
1. `librarian add lecture-recording.mp4` (audio extracted with FFmpeg, transcribed with Whisper, then ingested)
2. `librarian generate summary "lecture-recording"`

**Postconditions:** A concise summary of the lecture is generated and saved to `generated/`.
**Error flows:** Very large file hits Whisper limits → split the recording into parts.

### UC-005: Ingest a web article or YouTube lecture and chat with it

**Actor:** Self-learner
**Preconditions:** Groq API key configured.
**Flow:**
1. `librarian add https://example.com/article`
2. `librarian add https://youtube.com/watch?v=VIDEO_ID`
3. `librarian chat` — ask questions about the article/video

**Postconditions:** The article and video content are searchable and answerable within the bucket.
**Error flows:** URL unreachable or no extractable text → verify the link or try a different source.

### UC-006: Spaced-repetition review before an exam

**Actor:** Student
**Preconditions:** Flashcards/quiz items previously generated (which seed review items).
**Flow:**
1. `librarian review` — steps through items due for review in a spaced-repetition session.

**Postconditions:** Due items are reviewed and their schedules updated; if nothing is due, the tool prompts the student to generate flashcards.
**Error flows:** No items due → `librarian generate flashcards` first.

### UC-007: Search all materials for a specific topic or exercise number

**Actor:** Student
**Preconditions:** Materials ingested into the active bucket.
**Flow:**
1. `librarian search "exercise 0.3"` — query enhancement extracts the reference; hybrid semantic + keyword search returns the matching chunks, deduplicated.

**Postconditions:** Relevant passages (including the specific exercise) are surfaced from across the bucket.
**Error flows:** No matches → rephrase, or confirm the source containing it was ingested.

### UC-008: Test knowledge with an interactive quiz, then clean up

**Actor:** Student
**Preconditions:** Materials ingested.
**Flow:**
1. `librarian quiz` — answer multiple-choice, fill-in-the-blank, and short-answer questions with active recall and scoring.
2. `librarian list` to review documents; `librarian delete <id>` to remove outdated material.

**Postconditions:** Student gets immediate feedback; the collection is curated.
**Error flows:** Empty bucket → ingest materials before quizzing.

## Technical Context

- **Primary language:** Rust (edition 2021), async (Tokio).
- **Build system:** Cargo; binary `librarian`.
- **Retrieval:** local FastEmbed embeddings (all-MiniLM-L6-v2, cosine) + keyword
  matching, per-bucket SQLite storage.
- **LLM/transcription:** Groq (llama-3.3-70b-versatile / llama-3.1-8b-instant;
  whisper-large-v3-turbo, with whisper-large-v3 as the most-accurate option).

## Traceability

| Use Case | Requirement | Capability | Status |
|----------|-------------|------------|--------|
| UC-001 | R1, R2, R4 | C-001, C-002, C-004 | Shipped |
| UC-002 | R5 | C-005 | Shipped |
| UC-003 | R5 | C-005 | Shipped |
| UC-004 | R1, R5 | C-001, C-005 | Shipped |
| UC-005 | R1, R4 | C-001, C-004 | Shipped |
| UC-006 | R6 | C-006 | Shipped |
| UC-007 | R3 | C-003 | Shipped |
| UC-008 | R6, R3 | C-007, C-003 | Shipped |
