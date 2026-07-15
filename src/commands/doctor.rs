//! `librarian doctor` — environment health check (E8 / B-022).
//!
//! Reports, with actionable hints, whether everything The Librarian needs is in
//! place: a configured provider + credential, a writable data directory, the
//! optional media tools (FFmpeg/Tesseract), and the embedding model. Read-only —
//! it never changes config or data.

use std::io::Write;
use std::process::Command;

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;
use crate::embeddings;
use crate::llm::provider::ProviderKind;

/// One check outcome. `Warn` is for optional/degraded capabilities; `Fail` is a
/// problem that breaks a core feature.
enum Status {
    Ok,
    Warn,
    Fail,
}

pub fn run() -> Result<()> {
    println!("\n{}", "The Librarian — doctor".bold());
    println!("{}", "─".repeat(50).dimmed());

    let mut warns = 0usize;
    let mut fails = 0usize;
    let mut tally = |s: Status| match s {
        Status::Ok => {}
        Status::Warn => warns += 1,
        Status::Fail => fails += 1,
    };

    let config = Config::load().unwrap_or_default();

    // --- Config file ---
    match Config::config_path() {
        Ok(path) if path.exists() => {
            report(Status::Ok, "Config file", &path.display().to_string());
            tally(Status::Ok);
        }
        Ok(path) => {
            report(
                Status::Warn,
                "Config file",
                &format!("none yet ({}) — run `librarian config`", path.display()),
            );
            tally(Status::Warn);
        }
        Err(e) => {
            report(Status::Fail, "Config file", &e.to_string());
            tally(Status::Fail);
        }
    }

    // --- Provider + credential (chat / generation) ---
    let kind = config.provider_kind();
    let model = config.resolved_model();
    if config.has_api_key() {
        let detail = match kind {
            ProviderKind::Ollama => format!("{kind} · {model} (local, no key needed)"),
            _ => format!("{kind} · {model} · credential found"),
        };
        report(Status::Ok, "LLM provider", &detail);
        tally(Status::Ok);
    } else {
        report(
            Status::Fail,
            "LLM provider",
            &format!("{kind} · {model} · no credential — chat/generation disabled"),
        );
        tally(Status::Fail);
    }

    // --- Transcription (audio/video) ---
    match config.resolve_transcriber() {
        Ok(_) => {
            report(Status::Ok, "Transcription", "Groq/OpenAI key present");
            tally(Status::Ok);
        }
        Err(_) => {
            report(
                Status::Warn,
                "Transcription",
                "no Groq/OpenAI key — audio/video ingestion disabled",
            );
            tally(Status::Warn);
        }
    }

    // --- Data directory (writable) ---
    match Config::data_dir() {
        Ok(dir) => {
            if dir_writable(&dir) {
                report(Status::Ok, "Data directory", &dir.display().to_string());
                tally(Status::Ok);
            } else {
                report(
                    Status::Fail,
                    "Data directory",
                    &format!("not writable: {}", dir.display()),
                );
                tally(Status::Fail);
            }
        }
        Err(e) => {
            report(Status::Fail, "Data directory", &e.to_string());
            tally(Status::Fail);
        }
    }

    // --- Embedding model ---
    report(
        Status::Ok,
        "Embedding model",
        &format!(
            "{} ({}-dim, local; ~90MB downloaded on first use)",
            embeddings::MODEL_ID,
            embeddings::DIM
        ),
    );
    tally(Status::Ok);

    // --- Optional external tools ---
    if binary_present("ffmpeg", &["-version"]) {
        report(Status::Ok, "FFmpeg", "found (audio/video ingestion)");
        tally(Status::Ok);
    } else {
        report(
            Status::Warn,
            "FFmpeg",
            "not found — audio/video ingestion unavailable",
        );
        tally(Status::Warn);
    }

    if binary_present("tesseract", &["--version"]) {
        report(Status::Ok, "Tesseract", "found (image OCR)");
        tally(Status::Ok);
    } else {
        report(
            Status::Warn,
            "Tesseract",
            "not found — image OCR unavailable",
        );
        tally(Status::Warn);
    }

    // --- Summary ---
    println!("{}", "─".repeat(50).dimmed());
    if fails == 0 && warns == 0 {
        println!("{} Everything looks good.", "✓".green().bold());
    } else {
        println!(
            "{} {} problem(s), {} optional capability(ies) unavailable.",
            if fails > 0 {
                "✗".red()
            } else {
                "⚠".yellow()
            },
            fails,
            warns
        );
    }
    Ok(())
}

/// Print one aligned status row.
fn report(status: Status, label: &str, detail: &str) {
    let glyph = match status {
        Status::Ok => "✓".green(),
        Status::Warn => "⚠".yellow(),
        Status::Fail => "✗".red(),
    };
    println!(
        "  {} {:<16} {}",
        glyph,
        format!("{label}:").dimmed(),
        detail
    );
}

/// Whether `dir` can be created and written to (probes with a temp file).
fn dir_writable(dir: &std::path::Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".librarian-doctor-probe");
    match std::fs::File::create(&probe).and_then(|mut f| f.write_all(b"ok")) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Whether an external binary can be executed (present on PATH).
fn binary_present(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}
