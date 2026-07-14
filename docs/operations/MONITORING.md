---
title: "MONITORING"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Monitoring
author: Derek Martinez
---

# Monitoring — The Librarian

The Librarian is a **local CLI tool**. There is no server, no uptime to track, no fleet to alert on, and — by design — **no centralized telemetry or analytics**. "Monitoring" here means the observability that exists on the user's own machine and the lightweight practices a user or maintainer can follow. Absence of remote telemetry is an intentional privacy decision, consistent with the "privacy-first" posture in the README and `docs/operations/SECURITY_MODEL.md`.

## What Observability Exists

| Mechanism | How to use it | What it tells you |
|-----------|---------------|-------------------|
| Debug logging | `RUST_LOG=debug cargo run` (or on the installed binary) | Verbose runtime detail for troubleshooting ingestion, API calls, and search |
| Progress bars | `indicatif` progress bars during ingestion/embedding | Live progress on long operations (batch import, embedding, transcription) |
| Status dashboard | Interactive menu / status view (`librarian` with no args) | Current bucket, document counts, whether the API key is configured, library shelf |
| Contextual errors | `anyhow`-wrapped errors printed to stderr | Human-readable failure cause with context (which file, which step, Groq status code) |
| Process exit code | Shell `$?` / `%ERRORLEVEL%` | Non-zero on failure — usable in scripts |

## "Health" for a Local CLI

There is no `/healthz` endpoint. Health is simply: *can the user do the thing they want?* Practical checks:

1. **Does it build and run?** `cargo build --release` succeeds; `librarian` launches.
2. **Is the API key configured?** The status dashboard reports this; internally `Config::has_api_key()` checks both `config.toml` and the `GROQ_API_KEY` env var. Without it, chat/generate/transcription fail with "No API key configured".
3. **Is the embedding model available?** On first run fastembed downloads the all-MiniLM-L6-v2 ONNX model (~90MB) into the cache. A slow first run is expected; subsequent runs are fast.
4. **Are optional binaries present?** `check_ffmpeg()` probes for FFmpeg before video/audio work; Tesseract is required for image OCR. Missing tools produce a clear install-instruction error rather than a crash.
5. **Can it reach Groq?** Any network/auth/rate-limit problem surfaces as a `Groq API error (<status>): <body>` message.

## Logging Conventions

- **Primary language:** Rust (edition 2024).
- Diagnostic output goes to **stderr**; streamed LLM tokens and user-facing content go to **stdout**.
- Log verbosity is controlled by `RUST_LOG` (e.g. `debug`).
- **Never log secrets** (Rule R-8.2): the Groq API key is only used in the `Authorization` header and is never printed, including at debug level.

## Recommended Lightweight Practices

- **In scripts, check exit codes** rather than parsing output — a non-zero exit means the command failed.
- **Watch for `Groq API error (...)` messages** — status `401` means a bad/rotated key, `429` means rate-limited, `5xx` means a Groq-side outage (see `docs/operations/INCIDENT_RESPONSE.md`).
- **Reproduce bugs with `RUST_LOG=debug`** and capture the full stderr context when filing a GitHub issue.
- **Keep an eye on first-run model download** on slow or metered connections.

## Intentionally Absent

| Not present | Why |
|-------------|-----|
| Remote telemetry / analytics / phone-home | Privacy-first; nothing about the user's activity leaves the machine |
| Metrics backend, dashboards (Grafana, etc.) | No server; nothing to scrape |
| Alerting (PagerDuty, Slack, on-call rotation) | Single-user local tool; there is no operator to page |
| Uptime / latency / error-rate SLOs | No hosted service level to meet |
