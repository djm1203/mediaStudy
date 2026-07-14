---
title: "DEPLOYMENT"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-07-13T00:00:00Z
product_id: project
project_id: project
file_kind: Deployment
author: Derek Martinez
---

# Deployment — The Librarian

The Librarian is a **local, end-user CLI** — there is no server, no cloud
deployment, and no staging/production topology. "Deployment" means producing a
self-contained binary and getting it onto a user's `PATH`. State is stored locally
in per-bucket bundled SQLite databases; the only external service is the Groq API
(consumed at runtime, not hosted by this project).

## Distribution model

- **Single self-contained binary:** `cargo build --release` produces
  `target/release/librarian`. The release profile enables `lto = true` and
  `strip = true` for a smaller, optimized binary.
- **No system database:** SQLite is compiled in via `rusqlite` (`bundled`
  feature) — nothing to install or provision.
- **First-run download:** on first use the app fetches the local
  `all-MiniLM-L6-v2` embedding model (~90 MB) via `fastembed`. This is a one-time
  runtime download, not part of the build.
- **Optional runtime dependencies:** FFmpeg (audio/video transcription) and
  Tesseract (image OCR). These are only needed for those media types and are not
  bundled — the app degrades gracefully without them.

## Build for Distribution

```bash
git clone https://github.com/djm1203/mediaStudy.git
cd mediaStudy
cargo build --release
# Binary: ./target/release/librarian  (librarian.exe on Windows)
```

## Install (from source)

### Linux / macOS
```bash
cargo build --release
sudo cp target/release/librarian /usr/local/bin/
```

### Windows
```powershell
cargo build --release
# Copy target\release\librarian.exe to a directory on your PATH
```

Optional dependencies (per README):

- **Linux (Arch):** `sudo pacman -S ffmpeg tesseract tesseract-data-eng`
- **Linux (Ubuntu/Debian):** `sudo apt install ffmpeg tesseract-ocr tesseract-ocr-eng`
- **macOS:** `brew install ffmpeg tesseract`
- **Windows:** download FFmpeg (ffmpeg.org) and Tesseract (UB-Mannheim), add both
  to `PATH`.

## Runtime configuration

- Config file `config.toml` under the OS config dir (resolved via `dirs`), in the
  `librarian` app folder (migrated automatically from the legacy `media-study` folder):
  - Linux: `~/.config/librarian/config.toml`
  - macOS: `~/Library/Application Support/librarian/config.toml`
  - Windows: `%APPDATA%\librarian\config.toml`
- Keys: `groq_api_key`, `default_model`, `current_bucket`.
- Alternatively set `GROQ_API_KEY` in the environment. **No secrets are committed.**
- Data (per-bucket `documents.db` and generated materials) lives under the OS data
  dir alongside the config location.

## Release Automation (CI)

`.github/workflows/release.yml` triggers on a `v*` tag push (or manual
`workflow_dispatch`) and builds a cross-platform matrix on
`dtolnay/rust-toolchain@stable`:

| Target | Runner |
|--------|--------|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` (installs `libssl-dev`, `pkg-config`) |
| `x86_64-apple-darwin` | `macos-latest` |
| `aarch64-apple-darwin` | `macos-latest` |
| `x86_64-pc-windows-msvc` | `windows-latest` |

Each job runs `cargo build --release --target <target>`, packages the binary
(`.tar.gz` on Unix via `tar`, `.zip` on Windows via `7z`), uploads it as an
artifact, and a final `release` job attaches all artifacts to a GitHub Release
(`softprops/action-gh-release@v1`, `generate_release_notes: true`,
`GITHUB_TOKEN`).

> **Known gap — release.yml packages the wrong binary name.** The packaging steps
> reference `media-study` / `media-study.exe`, but the actual binary produced by
> `Cargo.toml` is **`librarian`** (`[[bin]] name = "librarian"`). As written, the
> `tar`/`7z` steps will fail (or archive nothing) because `media-study` does not
> exist. The workflow must be updated to package `librarian` /`librarian.exe` (and
> ideally the artifact names updated to match) before it can produce working
> releases. This is a real defect, not a stub — the CI (`ci.yml`) build/test path
> is sound, but the release path has not been exercised successfully.

## Rollback

Because distribution is a versioned binary with no live service, "rollback" means
distributing the previous tagged release binary. Local user data (SQLite buckets)
is forward-owned by the user and is not migrated or destroyed by swapping binaries;
schema changes must stay backward-compatible with existing databases (see
`API_CONTRACT.md`).
</content>
