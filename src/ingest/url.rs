use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::net::IpAddr;
use url::Url;

/// Extracted content from a URL
#[derive(Debug, Clone)]
pub struct UrlContent {
    pub url: String,
    pub title: String,
    pub text: String,
}

/// Validate URL for SSRF protection
fn validate_url(url: &Url) -> Result<()> {
    // Only allow http/https schemes
    match url.scheme() {
        "http" | "https" => {}
        scheme => anyhow::bail!("Unsupported URL scheme: {}. Only http and https are allowed.", scheme),
    }

    // Check host
    let host = url.host_str().ok_or_else(|| anyhow::anyhow!("URL has no host"))?;

    // Block cloud metadata endpoints
    if host == "169.254.169.254" || host == "metadata.google.internal" {
        anyhow::bail!("Access to cloud metadata endpoints is not allowed");
    }

    // Block localhost variations
    let host_lower = host.to_lowercase();
    if host_lower == "localhost" || host_lower == "127.0.0.1" || host_lower == "::1" {
        anyhow::bail!("Access to localhost is not allowed");
    }

    // Try to parse as IP address and check for private ranges
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            anyhow::bail!("Access to private IP addresses is not allowed");
        }
    }

    Ok(())
}

/// Check if an IP address is in a private range
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_private()           // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                || ipv4.is_loopback()   // 127.0.0.0/8
                || ipv4.is_link_local() // 169.254.0.0/16
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback() || ipv6.is_unspecified()
        }
    }
}

/// Fetch and extract readable content from a URL
pub async fn fetch_url(url_str: &str) -> Result<UrlContent> {
    let url = Url::parse(url_str).context("Invalid URL")?;

    // SSRF protection - validate URL before fetching
    validate_url(&url)?;

    // Check for YouTube URLs
    if is_youtube_url(&url) {
        return fetch_youtube_transcript(url_str).await;
    }

    // Fetch the page with redirect policy to prevent SSRF via redirects
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; media-study/0.1)")
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    let response = client
        .get(url_str)
        .send()
        .await
        .context("Failed to fetch URL")?;

    // Validate final URL after redirects
    let final_url = response.url();
    validate_url(final_url).context("Redirect led to blocked URL")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let html = response.text().await.context("Failed to read response")?;

    // Parse and extract content
    extract_article(&html, url_str)
}

/// Check if URL is a YouTube video
fn is_youtube_url(url: &Url) -> bool {
    let host = url.host_str().unwrap_or("");
    host.contains("youtube.com") || host.contains("youtu.be")
}

/// Extract article content from HTML
fn extract_article(html: &str, url: &str) -> Result<UrlContent> {
    let document = Html::parse_document(html);

    // Extract title
    let title = extract_title(&document).unwrap_or_else(|| url.to_string());

    // Try to find main content using common selectors
    let content_selectors = [
        "article",
        "main",
        "[role='main']",
        ".post-content",
        ".article-content",
        ".entry-content",
        ".content",
        "#content",
        ".post",
        ".article",
    ];

    let mut text = String::new();

    for selector_str in &content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let extracted = extract_text_from_element(&element);
                if extracted.len() > text.len() {
                    text = extracted;
                }
            }
        }

        // If we found substantial content, stop looking
        if text.len() > 500 {
            break;
        }
    }

    // Fallback: extract from body
    if text.len() < 200 {
        if let Ok(body_selector) = Selector::parse("body") {
            if let Some(body) = document.select(&body_selector).next() {
                text = extract_text_from_element(&body);
            }
        }
    }

    // Clean up the text
    text = clean_text(&text);

    if text.is_empty() {
        anyhow::bail!("Could not extract content from URL");
    }

    Ok(UrlContent { url: url.to_string(), title, text })
}

/// Extract title from document
fn extract_title(document: &Html) -> Option<String> {
    // Try og:title first
    if let Ok(selector) = Selector::parse("meta[property='og:title']") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let title = content.trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
        }
    }

    // Try <title> tag
    if let Ok(selector) = Selector::parse("title") {
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<String>();
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }

    // Try h1
    if let Ok(selector) = Selector::parse("h1") {
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<String>();
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }

    None
}

/// Extract text from an HTML element, filtering out scripts/styles
fn extract_text_from_element(element: &scraper::ElementRef) -> String {
    let mut text = String::new();

    // Tags to skip entirely
    let skip_tags = ["script", "style", "nav", "header", "footer", "aside", "noscript", "iframe"];

    for node in element.descendants() {
        match node.value() {
            scraper::Node::Text(t) => {
                // Check if any ancestor is a skip tag
                let mut should_skip = false;
                let mut current = node.parent();
                while let Some(parent) = current {
                    if let Some(elem) = parent.value().as_element() {
                        if skip_tags.contains(&elem.name()) {
                            should_skip = true;
                            break;
                        }
                    }
                    current = parent.parent();
                }

                if !should_skip {
                    let content = t.trim();
                    if !content.is_empty() {
                        if !text.is_empty() && !text.ends_with('\n') && !text.ends_with(' ') {
                            text.push(' ');
                        }
                        text.push_str(content);
                    }
                }
            }
            scraper::Node::Element(elem) => {
                // Add newlines for block elements
                if matches!(elem.name(), "p" | "br" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr") {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
            }
            _ => {}
        }
    }

    text
}

/// Clean up extracted text
fn clean_text(text: &str) -> String {
    let mut result = String::new();
    let mut prev_was_newline = false;
    let mut prev_was_space = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_was_newline && !result.is_empty() {
                result.push('\n');
                prev_was_newline = true;
            }
            continue;
        }

        // Skip common noise
        if trimmed.len() < 3 {
            continue;
        }

        if !result.is_empty() && !prev_was_newline {
            result.push('\n');
        }

        // Normalize spaces within the line
        for c in trimmed.chars() {
            if c.is_whitespace() {
                if !prev_was_space {
                    result.push(' ');
                    prev_was_space = true;
                }
            } else {
                result.push(c);
                prev_was_space = false;
            }
        }

        prev_was_newline = false;
    }

    result.trim().to_string()
}

/// Fetch YouTube transcript using yt-dlp
async fn fetch_youtube_transcript(url: &str) -> Result<UrlContent> {
    use tokio::process::Command;

    // Generate unique temp file prefix
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_secs();
    let pid = std::process::id();
    let temp_prefix = format!("media-study-yt-{}-{}", pid, timestamp);
    let temp_pattern = format!("/tmp/{}-%(id)s", temp_prefix);

    // First, get video info
    let info_output = Command::new("yt-dlp")
        .args(["--print", "title", "--no-download", url])
        .output()
        .await
        .context("yt-dlp not found. Install it with: pip install yt-dlp")?;

    let title = if info_output.status.success() {
        String::from_utf8_lossy(&info_output.stdout).trim().to_string()
    } else {
        "YouTube Video".to_string()
    };

    // Try to get auto-generated subtitles
    let output = Command::new("yt-dlp")
        .args([
            "--write-auto-sub",
            "--sub-lang", "en",
            "--skip-download",
            "--sub-format", "vtt",
            "-o", &temp_pattern,
            url,
        ])
        .output()
        .await
        .context("Failed to run yt-dlp")?;

    if !output.status.success() {
        // Try manual subtitles
        let output = Command::new("yt-dlp")
            .args([
                "--write-sub",
                "--sub-lang", "en",
                "--skip-download",
                "--sub-format", "vtt",
                "-o", &temp_pattern,
                url,
            ])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("No subtitles/transcript available for this video");
        }
    }

    // Find the subtitle file using async I/O
    let mut entries = tokio::fs::read_dir("/tmp").await?;
    let mut transcript_file = None;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with(&temp_prefix) && (name.ends_with(".vtt") || name.ends_with(".en.vtt")) {
                transcript_file = Some(path);
                break;
            }
        }
    }

    let transcript_path = transcript_file.context("Could not find downloaded transcript")?;

    // Parse VTT file using async I/O
    let vtt_content = tokio::fs::read_to_string(&transcript_path).await?;
    let text = parse_vtt(&vtt_content);

    // Clean up temp file (ignore errors)
    let _ = tokio::fs::remove_file(&transcript_path).await;

    if text.is_empty() {
        anyhow::bail!("Transcript was empty");
    }

    Ok(UrlContent {
        url: url.to_string(),
        title,
        text,
    })
}

/// Parse VTT subtitle format to plain text
fn parse_vtt(vtt: &str) -> String {
    let mut text = String::new();
    let mut seen_lines = std::collections::HashSet::new();

    for line in vtt.lines() {
        let line = line.trim();

        // Skip VTT header and timing lines
        if line.is_empty()
            || line.starts_with("WEBVTT")
            || line.starts_with("Kind:")
            || line.starts_with("Language:")
            || line.contains("-->")
            || line.chars().all(|c| c.is_ascii_digit() || c == ':' || c == '.' || c == ' ')
        {
            continue;
        }

        // Remove VTT tags like <c>, </c>, etc.
        let clean_line = remove_vtt_tags(line);
        let clean_line = clean_line.trim();

        if clean_line.is_empty() {
            continue;
        }

        // Deduplicate (auto-generated subs often repeat)
        if seen_lines.insert(clean_line.to_string()) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(clean_line);
        }
    }

    text
}

/// Remove VTT formatting tags
fn remove_vtt_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let input = "  Hello   world  \n\n\n  This is   a test  ";
        let output = clean_text(input);
        assert!(output.contains("Hello world"));
        assert!(output.contains("This is a test"));
    }

    #[test]
    fn test_is_youtube_url() {
        assert!(is_youtube_url(&Url::parse("https://www.youtube.com/watch?v=abc123").unwrap()));
        assert!(is_youtube_url(&Url::parse("https://youtu.be/abc123").unwrap()));
        assert!(!is_youtube_url(&Url::parse("https://example.com").unwrap()));
    }
}
