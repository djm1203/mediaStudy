//! Self-update check (B-023).
//!
//! Compares the running version against the latest GitHub release and reports
//! whether a newer build is available. This is a *check only* — it never
//! downloads or replaces the binary, so it needs no elevated permissions and
//! can't leave a half-installed executable behind. The user upgrades with the
//! install script or their package manager.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::io::Write;

const RELEASES_API: &str = "https://api.github.com/repos/djm1203/mediaStudy/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/djm1203/mediaStudy/releases/latest";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: Option<String>,
}

/// Query the latest release and print whether an update is available.
pub async fn check() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("{} v{current}", "The Librarian".bold());
    print!("Checking for updates... ");
    std::io::stdout().flush().ok();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        // GitHub's API requires a User-Agent header.
        .user_agent(concat!("librarian/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let release: Release = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("Failed to reach GitHub releases")?
        .error_for_status()
        .context("GitHub releases request failed")?
        .json()
        .await
        .context("Failed to parse GitHub release info")?;

    let latest = release.tag_name.trim_start_matches('v');

    if is_newer(latest, current) {
        println!("{}", "update available!".yellow().bold());
        println!("  Latest: v{latest}  (you have v{current})");
        println!(
            "  Download: {}",
            release.html_url.as_deref().unwrap_or(RELEASES_PAGE).cyan()
        );
    } else {
        println!("{}", "up to date.".green());
    }

    Ok(())
}

/// Is `latest` a strictly higher version than `current`? Compares
/// dot-separated numeric components; missing components count as 0.
fn is_newer(latest: &str, current: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    }
    let (l, c) = (parts(latest), parts(current));
    let n = l.len().max(c.len());
    for i in 0..n {
        let (lv, cv) = (
            l.get(i).copied().unwrap_or(0),
            c.get(i).copied().unwrap_or(0),
        );
        if lv != cv {
            return lv > cv;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn detects_newer_versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.0")); // numeric, not lexical
    }

    #[test]
    fn treats_equal_and_older_as_not_newer() {
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("0.1", "0.1.0")); // 0.1 padded == 0.1.0
        assert!(!is_newer("0.1.0", "0.1")); // symmetric: equal under padding
    }
}
