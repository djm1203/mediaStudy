---
title: "SCHEMA CHANGE POLICY"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: SchemaChangePolicy
author: Derek Martinez
---

# Schema Change Policy — The Librarian

## Context

The Librarian stores everything in **embedded SQLite** (`rusqlite`, `bundled`
feature). There is **no migration framework** and **no central schema version**.
Instead, schema is (re)created idempotently every time a database is opened:

- `storage/db.rs::init_schema` — `documents`, `documents_fts` (FTS5) + sync
  triggers, `conversations`, `messages`, `study_items`.
- `storage/chunks.rs::init_schema` — `chunks` + `idx_chunks_document_id`.

Every statement uses `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` /
`CREATE TRIGGER IF NOT EXISTS`, so opening an existing database never rewrites it.

Critically, **each bucket has its own database file** (`buckets/<name>/documents.db`,
plus a `default.db` for the no-bucket case). Any schema change must therefore work
across an unbounded number of independently-created DB files, each possibly created
by an older build of the binary. There is no single "production database" to
migrate — the user's whole library is the migration surface.

## Principles

- **Backward compatibility first.** New code must still open databases created by
  older versions. Because there is no version stamp, prefer changes that an old file
  can adopt automatically on next open.
- **Additive over destructive.** Add tables/columns/indexes; do not drop or rename
  existing ones without a real deprecation path.
- **Idempotent init.** Every new object is created with `IF NOT EXISTS` inside the
  relevant `init_schema`, so it materializes on the next open of any existing DB.
- **Review.** Schema changes get a second set of eyes (per BEACON governance) before
  merge.

## How to Add Schema Safely

### Add a column
SQLite `CREATE TABLE IF NOT EXISTS` will **not** alter an existing table, so a new
column will not appear in databases that already exist. To add one:

1. Add the column to the `CREATE TABLE` in `init_schema` (for fresh DBs).
2. For existing DBs, run a guarded `ALTER TABLE <t> ADD COLUMN <c> ...` that
   tolerates "duplicate column" errors (check `PRAGMA table_info(<t>)` first, or
   ignore the specific error). New columns **must be nullable or have a DEFAULT** —
   `ALTER TABLE ADD COLUMN` cannot add a `NOT NULL` column without a default.
3. Never assume the column exists on read; handle the absent/`NULL` case.

### Add a table or index
Add a `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` to the appropriate
`init_schema`. It will be created on the next open of every DB — no extra migration
needed.

### Change FTS
`documents_fts` is an FTS5 external-content table synced by triggers. If you change
its indexed columns you must update the three triggers (`documents_ai`, `documents_ad`,
`documents_au`) together and rebuild the index (`INSERT INTO documents_fts(documents_fts)
VALUES('rebuild')`) for existing DBs.

## Backward Compatibility Matrix

| Change | Compatible? | Approach |
|--------|-------------|----------|
| Add table / index / trigger | Yes | `CREATE ... IF NOT EXISTS` in `init_schema`. |
| Add nullable / DEFAULT column | Yes (with guarded ALTER) | Update `CREATE TABLE` **and** run guarded `ALTER TABLE ADD COLUMN` for existing DBs. |
| Add NOT NULL column | No | Add nullable, backfill, then enforce in code (SQLite can't add NOT NULL w/o default). |
| Drop / rename column or table | No | Deprecate: stop reading it, ship a release, only then remove. Rename = add-new + copy + drop-old. |
| Change embedding format/model | No | Invalidates stored vectors — requires re-embed (below). |

## The Embedding Coupling (special case)

`chunks.embedding` is a `BLOB` of the raw vector: 384 × `f32` serialized
little-endian (`embeddings::embedding_to_bytes` / `bytes_to_embedding`). These bytes
are meaningful **only** for the exact embedding model that produced them —
all-MiniLM-L6-v2 (384-dim) via `fastembed`.

Consequences:

- **Changing the embedding model (or its dimensionality) is a breaking data change.**
  Old BLOBs become semantically meaningless; cosine similarity against new query
  vectors would be garbage. A dimension change also breaks `cosine_similarity`,
  which returns 0.0 when lengths differ.
- **There is no stored model identifier.** If the model ever changes, add a way to
  detect stale vectors (e.g. a `model`/`embedding_version` column on `chunks`, or a
  metadata table) so a re-embed can be triggered.
- **Re-embed procedure:** re-run embedding for all chunks (the source `content` is
  retained in the `chunks` table, so re-embedding does not require re-ingesting the
  original files) and overwrite the `embedding` BLOBs. This must be applied per
  bucket database.

## Naming Conventions

- Tables: `snake_case`, plural (`documents`, `study_items`, `conversations`).
- Columns: `snake_case` (`created_at`, `document_id`, `next_review_date`).
- Indexes: `idx_<table>_<column>` (e.g. `idx_chunks_document_id`).
- Timestamps stored as RFC 3339 **TEXT** (parsed with `chrono`), not native types.

## Rollback

Without a migration framework, "rollback" means: an older binary must still open a
newer database. Additive changes (new tables/columns) are naturally forward- and
backward-tolerant because old code simply ignores unknown objects. Avoid any change
that would make an existing DB unopenable by a prior release; if unavoidable, gate it
behind a detectable marker and document it in `DECISIONS.md`.
