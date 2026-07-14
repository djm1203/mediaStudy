---
title: "PERFORMANCE BUDGET"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: PerformanceBudget
author: Derek Martinez
---

# Performance Budget — The Librarian

The Librarian is a local-first CLI, not a service. There is no P99 SLA and no
server fleet; "performance" means **interactive latency on one machine** plus the
network latency of the Groq API. Budgets below are guidance for a personal-scale
library (hundreds of documents, low tens of thousands of chunks), not hard gates.

## Where Time Goes

| Stage | Bound by | Dominant cost |
|-------|----------|---------------|
| Ingestion (text/PDF) | Local CPU + disk | PDF extraction; chunking is linear and cheap. |
| Ingestion (audio/video) | **Network** (upload + Whisper) | Uploading the file to Groq Whisper and waiting for transcription; video adds a local FFmpeg audio-extraction step. |
| Ingestion (image) | Local CPU (Tesseract) | OCR over the image. |
| Embedding | **Local CPU** | ONNX all-MiniLM-L6-v2 inference; first run also pays a one-time ~90 MB model download. |
| Retrieval | **Local CPU/memory** | Loads *all* chunk embeddings and brute-force cosine-scores them (O(n)). |
| Answer generation | **Network** (Groq chat) | Streamed LLM completion — the dominant latency of a `chat` turn. |

## Rough Targets (single machine, warm)

| Metric | Target / expectation | Notes |
|--------|----------------------|-------|
| First-run model download | one-time, ~90 MB | Cached locally; not paid again. |
| Embedding throughput | CPU-bound; scales ~linearly with chunk count | Batched via `fastembed`; hundreds of chunks per document embed in seconds on a modern CPU. |
| Retrieval (semantic) | sub-second up to ~tens of thousands of chunks | Brute-force cosine over in-memory `f32` vectors; grows linearly with total chunks in the bucket. |
| Keyword search | sub-second at personal scale | `content LIKE '%kw%'` is a full scan (no FTS index on chunks); acceptable while chunk counts are modest. |
| Chat turn (end-to-end) | ~seconds, network-dominated | Local retrieval is small relative to Groq round-trip + generation time. |
| Binary size | small single artifact | Release profile uses `lto = true` + `strip = true`. |

## Scaling Risk (the main ceiling)

Semantic retrieval is **brute-force**: `chunks.get_all_with_embeddings()` pulls every
embedding into memory and `embeddings::find_similar` scores each one on every query.
This is O(n) in the number of chunks in the active bucket, both in time and in memory
(each vector is 384 × 4 bytes ≈ 1.5 KB, plus the source text). At personal scale this
is instant. For very large libraries (hundreds of thousands to millions of chunks) it
becomes the bottleneck:

- **Memory:** all vectors are loaded per query.
- **CPU:** linear scan per query, single-threaded scoring.
- **Keyword arm:** `LIKE` is also a full table scan.

Mitigations to consider *if/when* libraries grow past comfortable brute-force size
(record any adoption in `DECISIONS.md`):

1. Use **per-bucket isolation** deliberately — keep each bucket scoped to one
   subject so a query only scans that bucket (this is already the design).
2. Add an **ANN index** (e.g. HNSW) or a vector extension to avoid the linear scan.
3. Promote chunk keyword search to **FTS5** (currently only `documents_fts` exists).
4. Cache query embeddings / avoid reloading all vectors per turn.

## Monitoring & Optimization Guidelines

- The tool already surfaces progress via `indicatif` spinners/bars during ingestion,
  embedding, and transcription — the natural place to notice regressions.
- Profile before optimizing; the two biggest levers are (a) the Groq round-trip
  (network, mostly out of our control beyond context sizing) and (b) the O(n)
  retrieval scan.
- Context sent to the LLM is already dynamically sized and clamped
  (`available_context_chars(...).clamp(2000, 30000)`) to balance answer quality
  against token cost/latency — keep that clamp in mind before enlarging context.
- Document any performance trade-off in `DECISIONS.md`; add regression coverage for
  chunking/search hot paths (the chunker and `search` module already have unit tests).
