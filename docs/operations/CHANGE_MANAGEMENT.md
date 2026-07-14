---
title: "CHANGE MANAGEMENT"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: ChangeManagement
author: Derek Martinez
---

# Change Management — The Librarian

The Librarian is an open-source, local Rust CLI maintained by a small author base. Change management is proportionate to that: git-based, PR-reviewed, CI-gated, and governed by the BEACON session protocol in `CLAUDE.md`. There is no deployment fleet — a "release" is a set of prebuilt binaries produced by CI from a tag.

## Change Classification

| Class | Description | Review | Notes |
|-------|-------------|--------|-------|
| Standard | Docs, comments, formatting, trivial fixes | Author self-review + CI | Low risk; still goes through a PR/branch |
| Normal | Features, bug fixes, dependency bumps | PR review + all CI gates green | Default path for most work |
| Schema | Changes to the SQLite schema (documents/chunks/embeddings) | PR review + follow `SCHEMA_CHANGE_POLICY` | Requires a migration/back-compat plan (see `docs/standards/`) |
| Emergency | Critical fix (e.g. leaked-secret helper, crash on all input, security patch) | Expedited PR; review post-hoc if needed | Fast-track, but still branch + CI |

## Change Process

1. **Work on a feature branch**, never directly on `main` (`git checkout -b feature/...` or `fix/...`).
2. **Make the change** following existing code patterns and the BEACON session protocol (start a session, update state docs).
3. **Run the local gates before pushing:**
   - `cargo fmt --all` (or `-- --check`)
   - `cargo clippy --all-features -- -D warnings`
   - `cargo test --all-features`
   - `cargo check --all-features`
4. **Open a PR to `main`.** CI (`.github/workflows/ci.yml`) runs the same four gates — **check, rustfmt, clippy, and test** — and all must pass before merge.
5. **Review and merge.** Address review feedback; merge once green.
6. **Update state docs** per the BEACON close protocol: `STATUS.md`, `HANDOFF.md`, `CHANGELOG.md`, `DECISIONS.md`, `OPEN_QUESTIONS.md`, `RISKS.md`.

## CI Gates (authoritative)

`.github/workflows/ci.yml` runs on every push and PR to `main`:

| Job | Command | Blocks merge |
|-----|---------|--------------|
| Check | `cargo check --all-features` | Yes |
| Rustfmt | `cargo fmt --all -- --check` | Yes |
| Clippy | `cargo clippy --all-features -- -D warnings` | Yes |
| Test | `cargo test --all-features` | Yes |

> Recommended addition (not yet present): a `cargo audit` job for dependency-vulnerability scanning — see `docs/compliance/VULNERABILITY_POLICY.md`.

## Commit Policy (R-10.5, from CLAUDE.md)

- **Never commit autonomously.** Create a commit only when the operator explicitly asks.
- **No AI attribution.** No `Co-Authored-By`, "Generated with", "written by", or any trailer/footer naming an AI agent, model, or tool.
- **Keep messages concise:** short subject, at most a sentence or two of body describing what changed and why. Detail belongs in the state docs, not the commit message.
- **Never push without instruction** (R-10.2).

## Versioning & Releases

- The crate uses **semantic-ish versioning** in `Cargo.toml` (currently `the-librarian` `v0.1.0`). Bump the version deliberately for user-facing changes; pre-`1.0`, minor/patch semantics are loose but breaking changes should still bump conspicuously.
- **Releases are tag-driven:** pushing a `v*` tag triggers `.github/workflows/release.yml`, which cross-compiles binaries for Linux x86_64, macOS x86_64 + aarch64, and Windows x86_64, packages them (`.tar.gz` / `.zip`), and creates a GitHub Release with auto-generated notes.
- Record the release in `docs/state/CHANGELOG.md`.

## Schema Changes

Any change to the per-bucket SQLite schema (documents, chunks, embeddings, FTS5 index) is a **Schema-class change**: follow `SCHEMA_CHANGE_POLICY` in `docs/standards/`, provide a migration or back-compat path for existing user databases, and prefer additive changes so already-ingested buckets keep working without a full re-ingest.

## Audit Trail

- All changes are tracked in git; PRs are the unit of review.
- Decisions are recorded in `docs/state/DECISIONS.md`; changes in `docs/state/CHANGELOG.md`.
- Session lifecycle is governed by the BEACON protocol in `CLAUDE.md`.
