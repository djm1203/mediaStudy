---
title: "INCIDENT RESPONSE"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: IncidentResponse
author: Derek Martinez
---

# Incident Response — The Librarian

The Librarian is a **local, single-user CLI**. "Incidents" are not production outages — there is no production. They are problems an individual user hits on their own machine, plus maintenance events the maintainer handles for the open-source project. This document gives right-sized playbooks for both.

## Severity (proportionate)

| Level | Meaning for this tool | Examples |
|-------|-----------------------|----------|
| High | Secret exposure, or the tool is unusable for its core purpose | Leaked Groq API key; tool crashes on all input |
| Medium | A major feature is degraded but workarounds exist | Groq outage (chat/transcription down, local search still works); one bucket's DB corrupted |
| Low | Minor/cosmetic or single-item failure | One malformed PDF fails to ingest; UI glitch |

## Reporting

- **Users and contributors:** open a **GitHub issue** at the project repository (`https://github.com/djm1203/mediaStudy`). Include OS, `librarian`/crate version, the command run, and `RUST_LOG=debug` output. **Do not paste your API key or private study content.**
- **Suspected security vulnerabilities:** follow `docs/compliance/VULNERABILITY_POLICY.md` (use a GitHub private security advisory rather than a public issue).

## Playbooks

### (a) Leaked Groq API key
**Signal:** key committed to git, pasted in a screenshot/issue, synced to a shared location, or unexpected Groq usage/billing.

1. **Rotate immediately** at `https://console.groq.com/` — revoke the exposed key and generate a new one. Rotation is the real fix; the old key is compromised the moment it leaks.
2. Update the local secret: run `librarian config` to store the new key, or update `GROQ_API_KEY`. Remove the old value from `config.toml`.
3. **Scrub traces:** clear the old key from shell history (`~/.bash_history`, `~/.zsh_history`, PowerShell `ConsoleHost_history.txt`), and from any issue/PR/screenshot where it appeared.
4. If it was ever committed, treat the key as burned (rotation already handled that) and remove it from history (e.g. `git filter-repo`) before pushing anywhere public.
5. **Prevention:** prefer the `GROQ_API_KEY` env var; keep `config.toml` out of the repo (already in `.gitignore`); rely on the `0o600` perms set on Unix.

### (b) Groq API outage or rate-limit
**Signal:** `Groq API error (429 ...)` (rate-limited) or `Groq API error (5xx ...)` / network failure.

1. **Confirm scope:** local features are unaffected — `add` (non-audio), `list`, `search`, and reading stored content work fully offline. Only chat, generation, and transcription need Groq.
2. **429 (rate-limit):** wait and retry; reduce batch size; consider a lighter/faster model (`llama-3.1-8b-instant`) to lower per-call cost.
3. **5xx / network:** check `https://console.groq.com/` status and your connectivity; retry with backoff.
4. **Degraded-mode workaround:** keep using local semantic search to find and read relevant material until the API recovers.
5. **Follow-up (maintainer):** if transient failures are common, consider adding automatic retry/backoff around the Groq HTTP calls in `src/llm/groq.rs` / `whisper.rs`.

### (c) Corrupted bucket SQLite database
**Signal:** a bucket fails to open, or queries error on `buckets/<name>/documents.db`.

1. **Blast radius is contained** by design: each bucket has its own DB (`buckets/<name>/documents.db`), so corruption in one bucket does not affect others or `default.db`.
2. Try SQLite recovery on the affected file (e.g. `sqlite3 documents.db ".recover" | sqlite3 recovered.db`) if the data matters.
3. If recovery fails, **rebuild by re-ingesting**: the source materials are the user's original files. Delete/recreate the bucket and re-run `librarian add` on the sources. Chunks/embeddings are regenerated locally.
4. Generated artifacts in `buckets/<name>/generated/` can be regenerated with `librarian generate`.
5. **Prevention:** users concerned about durability should keep their original source files and/or back up the OS data dir.

### (d) Vulnerable dependency / CVE in a crate
**Signal:** `cargo audit` advisory, GitHub Dependabot alert, or upstream disclosure.

1. Triage per `docs/compliance/VULNERABILITY_POLICY.md` (is the vulnerable code path reachable from this tool?).
2. Update the crate (`cargo update -p <crate>` or bump the version in `Cargo.toml`), rebuild, and run the CI gates (`cargo check/fmt/clippy/test`).
3. If no fix is available, evaluate a workaround or replacing the dependency.
4. Cut a new release via `.github/workflows/release.yml` (tag `v*`) so users get the patched binary; note it in `docs/state/CHANGELOG.md`.

### (e) Crash on malicious/malformed input
**Signal:** a specific PDF/URL/media file causes a panic or unexpected failure.

1. **Capture a reproducer:** the offending file/URL (sanitized if sensitive) and the `RUST_LOG=debug` output.
2. Note that PDF parsing is already panic-guarded with a `pdf-extract` → `lopdf` fallback (`src/ingest/pdf.rs`); a new crash likely indicates a gap in another ingest path (`url.rs`, `ocr.rs`, `text.rs`) or a fallback that also fails.
3. Fix the parser/handler to return a clean `anyhow` error instead of crashing.
4. **Add a regression test** with the minimal reproducer so the class of bug cannot recur silently.
5. Release the fix (playbook d, step 4).

## Post-Incident (for the maintainer)

For any High or recurring Medium incident, follow the BEACON pipeline below so the fix hardens the framework, not just the code.

## Incident → BEACON Rule Pipeline

Every High-severity (and any recurring Medium) incident must produce at least one of the following before it is closed:

1. **A new rule** in `docs/governance/RULESET.md` that would have caught this class of incident.
2. **An update to an existing rule** that proved insufficient.
3. **A new checklist item** in `docs/standards/DEFINITION_OF_DONE.md` if a pre-ship check was missing (e.g. "run `cargo audit`", "add a reproducer test for parser crashes").
4. **A documented exception** in `docs/state/DECISIONS.md` if the incident reveals a known, accepted risk (e.g. plaintext API key storage).

Log the rule change in `docs/state/DECISIONS.md`:

```
## D-[N]: Rule added/updated following [incident name]

**Date:** YYYY-MM-DD
**Incident:** [severity and brief description]
**Root cause:** [what rule or check was missing]
**Change made:** [which rule was added or updated and what it now says]
**Rationale:** [why this prevents recurrence]
```

**The goal:** every incident makes the framework smarter. A review that produces only action items but no rule change leaves the same class of mistake possible next time.
