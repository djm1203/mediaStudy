//! Build script: embed lightweight version metadata (git short SHA + commit
//! date) into the binary so `librarian --version` reports the exact build
//! (B-006). Falls back to "unknown" when git is unavailable (e.g. a crates.io
//! source build), so it never breaks the build.

use std::process::Command;

fn main() {
    let sha = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let date = git(&["log", "-1", "--format=%cd", "--date=short"])
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=LIBRARIAN_GIT_SHA={sha}");
    println!("cargo:rustc-env=LIBRARIAN_BUILD_DATE={date}");

    // Re-run when the checked-out commit changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}

/// Run a git subcommand, returning its trimmed stdout on success.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}
