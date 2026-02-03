# Media Study

```
  __  __          _ _       ____  _             _
 |  \/  | ___  __| (_) __ _/ ___|| |_ _   _  __| |_   _
 | |\/| |/ _ \/ _` | |/ _` \___ \| __| | | |/ _` | | | |
 | |  | |  __/ (_| | | (_| |___) | |_| |_| | (_| | |_| |
 |_|  |_|\___|\__,_|_|\__,_|____/ \__|\__,_|\__,_|\__, |
                                                  |___/
```

A powerful CLI tool for ingesting various media types and studying with LLM-powered assistance. Create isolated knowledge "buckets" for different subjects, generate study guides, flashcards, quizzes, and get AI-powered answers grounded in your source materials.

## Features

- **Multi-format ingestion**: PDFs, text files, Markdown, audio, video, images (OCR), web articles, YouTube videos
- **Knowledge isolation**: Organize materials into separate "buckets" (per class/project)
- **Semantic search**: Local vector embeddings (all-MiniLM-L6-v2) with cosine similarity
- **Study tools**: Generate study guides, flashcards, quizzes, and summaries
- **Interactive chat**: Ask questions grounded in your ingested materials
- **Homework help**: Guided problem-solving mode
- **Cross-platform**: Single binary for Windows, macOS, and Linux
- **Privacy-first**: Embeddings generated locally, only LLM queries sent to API

## Prerequisites

### Required
- **Groq API Key**: Sign up free at [console.groq.com](https://console.groq.com/)

### Optional (for specific media types)

| Tool | Purpose | Installation |
|------|---------|--------------|
| **ffmpeg** | Video/audio processing | `apt install ffmpeg` / `brew install ffmpeg` |
| **tesseract** | Image OCR | `apt install tesseract-ocr` / `brew install tesseract` |
| **yt-dlp** | YouTube transcripts | `pip install yt-dlp` |

## Installation

### From Source

```bash
git clone https://github.com/yourusername/mediaStudy.git
cd mediaStudy
cargo install --path .
```

### Build for Development

```bash
cargo build --release
./target/release/media-study
```

## Quick Start

```bash
# 1. Configure your API key
media-study config

# 2. Create a knowledge bucket
media-study bucket create "CS 101"

# 3. Add study materials
media-study add lecture-notes.pdf
media-study add https://example.com/article

# 4. Chat with your materials
media-study chat

# 5. Generate study materials
media-study generate
```

## Usage

### Interactive Mode

Just run `media-study` with no arguments for the interactive menu:

```bash
media-study
```

### Commands

```bash
# Content Management
media-study add <path/url>        # Add files, directories, or URLs
media-study list                   # List all documents
media-study search <query>         # Search documents
media-study docs                   # Manage documents (view/delete)

# Study Tools
media-study chat                   # Interactive Q&A with your materials
media-study generate study-guide   # Generate comprehensive study guide
media-study generate flashcards    # Generate flashcards
media-study generate quiz          # Generate practice quiz
media-study generate summary       # Generate summary
media-study generate homework      # Interactive homework help

# Organization
media-study bucket create <name>   # Create a new bucket
media-study bucket list            # List all buckets
media-study bucket use <name>      # Switch to a bucket
media-study bucket delete <name>   # Delete a bucket

# Configuration
media-study config                 # Configure API key and settings
```

### Adding Content

```bash
# Files
media-study add textbook.pdf
media-study add notes.md
media-study add lecture.mp3

# Directories (batch import)
media-study add ./course-materials/

# URLs
media-study add https://example.com/article
media-study add https://youtube.com/watch?v=VIDEO_ID

# Images (OCR)
media-study add diagram.png
```

### Supported Formats

| Category | Extensions |
|----------|------------|
| Documents | `.pdf`, `.txt`, `.md` |
| Audio | `.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac` |
| Video | `.mp4`, `.mkv`, `.avi`, `.mov`, `.webm` |
| Images | `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.tiff` |
| Web | Any `http://` or `https://` URL |

## Configuration

Configuration is stored at:
- **Linux/macOS**: `~/.config/media-study/config.toml`
- **Windows**: `%APPDATA%\media-study\config.toml`

```toml
groq_api_key = "gsk_..."
default_model = "openai/gpt-oss-120b"
current_bucket = "CS-101"
```

Environment variable alternative:
```bash
export GROQ_API_KEY="gsk_..."
```

## Shell Completions

Generate shell completions for your preferred shell:

```bash
# Bash (add to ~/.bashrc)
media-study completions bash >> ~/.bashrc

# Zsh (add to ~/.zshrc)
media-study completions zsh >> ~/.zshrc

# Fish
media-study completions fish > ~/.config/fish/completions/media-study.fish

# PowerShell
media-study completions powershell >> $PROFILE
```

## How It Works

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Ingest    │────▶│   Chunk &    │────▶│   Store     │
│  (PDF/URL)  │     │   Embed      │     │  (SQLite)   │
└─────────────┘     └──────────────┘     └─────────────┘
                                                │
┌─────────────┐     ┌──────────────┐     ┌──────▼──────┐
│  Response   │◀────│   Groq LLM   │◀────│  Semantic   │
│  (Grounded) │     │   + Context  │     │   Search    │
└─────────────┘     └──────────────┘     └─────────────┘
```

1. **Ingestion**: Extract text from various formats
2. **Chunking**: Split into ~1000 char chunks with 200 char overlap
3. **Embedding**: Generate vectors locally (all-MiniLM-L6-v2)
4. **Storage**: SQLite database per bucket
5. **Search**: Cosine similarity on query embedding
6. **Generation**: Send relevant context to Groq LLM

## Models

| Purpose | Model | Notes |
|---------|-------|-------|
| Embeddings | all-MiniLM-L6-v2 | Local, fast, 384-dim |
| Transcription | whisper-large-v3 | Via Groq API |
| Chat/Generation | openai/gpt-oss-120b | High quality (default) |
| Fast queries | llama-3.1-8b-instant | Lower latency |

## Project Structure

```
src/
├── main.rs           # CLI entry point
├── config.rs         # Configuration management
├── bucket/           # Knowledge bucket isolation
├── commands/         # CLI command implementations
├── embeddings/       # Local embedding generation
├── ingest/           # Media ingestion (PDF, URL, OCR, etc.)
├── llm/              # LLM clients (Groq, Whisper)
└── storage/          # SQLite storage layer
```

## Development

```bash
# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run

# Check for issues
cargo clippy

# Format code
cargo fmt
```

## Troubleshooting

### "No API key configured"
```bash
media-study config
# Or set environment variable:
export GROQ_API_KEY="gsk_..."
```

### "ffmpeg not found"
```bash
# Ubuntu/Debian
sudo apt install ffmpeg

# macOS
brew install ffmpeg
```

### "tesseract not found"
```bash
# Ubuntu/Debian
sudo apt install tesseract-ocr

# macOS
brew install tesseract
```

### Slow first run
The embedding model (~90MB) is downloaded on first use. Subsequent runs are fast.

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Groq](https://groq.com/) - Fast LLM inference
- [fastembed](https://github.com/Anush008/fastembed-rs) - Local embeddings
- [clap](https://github.com/clap-rs/clap) - CLI framework
- [inquire](https://github.com/mikaelmello/inquire) - Interactive prompts
