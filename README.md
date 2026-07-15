# The Librarian

```
╔════════════════════════════════════════════════════════╗
║                                                        ║
║   ▀█▀ █ █ █▀▀   █   █ █▄▄ █▀█ ▄▀█ █▀█ █ ▄▀█ █▄ █     ║
║    █  █▀█ ██▄   █▄▄ █ █▄█ █▀▄ █▀█ █▀▄ █ █▀█ █ ▀█     ║
║                                                        ║
║            ┌─────────────────────────────┐             ║
║            │  📚 Your Study Companion 📚  │             ║
║            └─────────────────────────────┘             ║
╚════════════════════════════════════════════════════════╝
```

Your personal AI study companion. Ingest PDFs, lecture recordings, notes, and web articles into organized "books" (knowledge buckets), then chat with your materials, generate study guides, flashcards, quizzes, and get AI-powered answers grounded in your source content.

## Features

- **Multi-format ingestion**: PDFs, text files, Markdown, audio, video, images (OCR), web articles, YouTube videos
- **Library organization**: Organize materials into separate "books" (buckets) per class/project
- **Hybrid retrieval**: Local vector embeddings (all-MiniLM-L6-v2, cosine similarity) fused with an FTS5 keyword index via Reciprocal Rank Fusion, with structure-aware chunking
- **Verifiable citations**: Grounded answers cite numbered sources that map back to a specific document and chunk, shown in a "Sources" view
- **Pluggable providers**: Groq (default), OpenAI, Anthropic, or a local **Ollama** server for fully offline chat — selectable in the TUI Config screen
- **Study tools**: Generate study guides, flashcards, quizzes, and summaries - saved to your library
- **Interactive chat**: Ask "The Librarian" questions grounded in your ingested materials
- **Full-screen TUI**: A ratatui terminal UI (Home, Chat, Search, Docs, Add, Study, Quiz, Review, Config) with streaming chat; `librarian <cmd> <args>` stays headless for scripting
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Privacy-first**: Embeddings generated locally; only LLM queries leave the machine (and nothing does with an Ollama provider)

## Prerequisites

### Required
- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **An LLM provider** (for chat, generation, and transcription):
  - **Groq API Key** (default) — sign up free at [console.groq.com](https://console.groq.com/), or
  - an **OpenAI** or **Anthropic** API key, or
  - a local **[Ollama](https://ollama.com/)** server for offline chat/generation (no key needed; transcription still needs Groq/OpenAI)

### Optional (for specific media types)

| Tool | Purpose | Installation |
|------|---------|--------------|
| **FFmpeg** | Video/audio transcription | See [installation](#installing-optional-dependencies) |
| **Tesseract** | Image/screenshot OCR | See [installation](#installing-optional-dependencies) |

## Installation

### One-line install (Linux/macOS)

Grab the latest prebuilt binary from GitHub releases:

```bash
curl -fsSL https://raw.githubusercontent.com/djm1203/mediaStudy/main/install.sh | sh
```

It installs to `~/.local/bin` (override with `LIBRARIAN_INSTALL_DIR`). On Windows, download
the `.zip` from the [releases page](https://github.com/djm1203/mediaStudy/releases/latest).
Check for newer versions any time with `librarian update`.

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/djm1203/mediaStudy.git
cd mediaStudy

# Build release binary
cargo build --release

# The binary is at ./target/release/librarian
# Optionally, copy to your PATH:
sudo cp target/release/librarian /usr/local/bin/
```

### Installing Optional Dependencies

#### Linux (Arch)
```bash
sudo pacman -S ffmpeg tesseract tesseract-data-eng
```

#### Linux (Ubuntu/Debian)
```bash
sudo apt install ffmpeg tesseract-ocr tesseract-ocr-eng
```

#### macOS
```bash
brew install ffmpeg tesseract
```

#### Windows
- **FFmpeg**: Download from [ffmpeg.org](https://ffmpeg.org/download.html), add to PATH
- **Tesseract**: Download from [UB-Mannheim](https://github.com/UB-Mannheim/tesseract/wiki), add to PATH

## Quick Start

```bash
# 1. Configure your API key
librarian config

# 2. Create a knowledge book (bucket)
librarian bucket create "PSC-4395"

# 3. Add study materials
librarian add lecture-notes.pdf
librarian add ~/Documents/SchoolDocs/PSC/

# 4. Chat with your materials
librarian chat

# 5. Generate study materials
librarian generate
```

## Usage

### Interactive Mode (Recommended)

Just run `librarian` with no arguments to launch the full-screen TUI:

```bash
librarian
```

You land on the Home screen with your library sidebar and a status dashboard. Navigate
screens with number keys `1`–`9` (Home, Chat, Search, Docs, Add, Study, Quiz, Review,
Config), `Tab` to move focus, `?` for help, `Ctrl-T` to toggle light/dark, and `q` to quit.
Running a subcommand with no argument (e.g. `librarian chat`, `librarian search`) opens that
screen directly; passing an argument keeps it headless for scripting.

### Commands

```bash
# Content Management
librarian add <path/url>           # Add files, directories, or URLs
librarian list                     # List all documents in current book
librarian search <query>           # Search documents
librarian docs                     # Manage documents (view/delete)

# Study Tools
librarian chat                     # Interactive Q&A with your materials (TUI)
librarian generate study-guide     # Generate comprehensive study guide
librarian generate flashcards      # Generate flashcards
librarian generate quiz            # Generate practice quiz
librarian generate summary         # Generate summary
librarian review                   # Spaced-repetition review session (TUI)
librarian quiz                     # Interactive quiz (TUI)

# Library Organization
librarian bucket create <name>     # Create a new book
librarian bucket list              # List all books
librarian bucket use <name>        # Switch to a book
librarian bucket delete <name>     # Delete a book
librarian library                  # Alias for bucket management

# Configuration
librarian config                   # Configure provider, API key, and model (TUI)
librarian completions <shell>      # Generate shell completions
librarian update                   # Check for a newer release
```

### Adding Content

```bash
# Single files
librarian add textbook.pdf
librarian add notes.md
librarian add lecture.mp3
librarian add screenshot.png        # Requires tesseract

# Directories (batch import)
librarian add ./course-materials/

# URLs
librarian add https://example.com/article
librarian add https://youtube.com/watch?v=VIDEO_ID

# Videos (requires ffmpeg)
librarian add lecture-recording.mp4
```

### Supported Formats

| Category | Extensions | Requirements |
|----------|------------|--------------|
| Documents | `.pdf`, `.txt`, `.md` | None |
| Audio | `.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac` | FFmpeg + API key |
| Video | `.mp4`, `.mkv`, `.avi`, `.mov`, `.webm` | FFmpeg + API key |
| Images | `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.tiff` | Tesseract |
| Web | Any `http://` or `https://` URL | None |

## Configuration

Configuration is stored at (auto-migrated from the legacy `media-study` directory):
- **Linux**: `~/.config/librarian/config.toml`
- **macOS**: `~/Library/Application Support/librarian/config.toml`
- **Windows**: `%APPDATA%\librarian\config.toml`

Pick a provider and set the matching key (easiest via the TUI **Config** screen). Example:

```toml
provider = "groq"                       # groq | openai | anthropic | ollama
default_model = "openai/gpt-oss-120b"    # optional; omit to use the provider default
current_bucket = "psc-4395"

# Only the key for your chosen provider is needed:
groq_api_key = "gsk_..."
# openai_api_key = "sk-..."
# anthropic_api_key = "sk-ant-..."
# ollama_url = "http://localhost:11434/v1"   # Ollama needs no key
```

Environment variable alternative (Groq):
```bash
export GROQ_API_KEY="gsk_..."
```

## Shell Completions

Generate shell completions for tab-completion support:

```bash
# Bash (add to ~/.bashrc)
librarian completions bash >> ~/.bashrc

# Zsh (add to ~/.zshrc)
librarian completions zsh >> ~/.zshrc

# Fish
librarian completions fish > ~/.config/fish/completions/librarian.fish

# PowerShell
librarian completions powershell >> $PROFILE
```

## How It Works

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Ingest    │────▶│   Chunk &    │────▶│   Store     │
│  (PDF/URL)  │     │   Embed      │     │  (SQLite)   │
└─────────────┘     └──────────────┘     └─────────────┘
                                                │
┌─────────────┐     ┌──────────────┐     ┌──────▼──────┐
│  Response   │◀────│  LLM provider│◀────│   Hybrid    │
│ (+ Sources) │     │   + Context  │     │  Retrieval  │
└─────────────┘     └──────────────┘     └─────────────┘
```

1. **Ingestion**: Extract text from various formats (PDF parsing, Whisper transcription, OCR)
2. **Chunking**: Structure-aware split (respects headings/paragraphs) targeting ~1000 chars with 200-char overlap
3. **Embedding**: Generate 384-dim vectors locally using all-MiniLM-L6-v2
4. **Storage**: SQLite database per book (bucket) with FTS5 full-text search on both documents and chunks
5. **Retrieval**: Hybrid search — semantic (cosine) + keyword (FTS5) arms fused with Reciprocal Rank Fusion
6. **Generation**: Send the top-ranked chunks as numbered context to the selected LLM provider; the answer cites those sources

## Models Used

| Purpose | Model | Notes |
|---------|-------|-------|
| Embeddings | all-MiniLM-L6-v2 | Local, ~90MB download on first run |
| Transcription | whisper-large-v3-turbo | Via Groq/OpenAI (default; `whisper-large-v3` for most accurate) |
| Chat/Generation (Groq, default) | `openai/gpt-oss-120b` | Provider default; any Groq model works |
| Chat/Generation (OpenAI) | `gpt-4o-mini` | Provider default |
| Chat/Generation (Anthropic) | `claude-sonnet-5` | Provider default |
| Chat/Generation (Ollama) | `llama3.1` | Local/offline default |

Set `default_model` in `config.toml` (or the TUI Config screen) to override any provider's default.

## Data Storage

Data is stored at (auto-migrated from the legacy `media-study` directory):
- **Linux**: `~/.local/share/librarian/`
- **macOS**: `~/Library/Application Support/librarian/`
- **Windows**: `%APPDATA%\librarian\`

Structure:
```
librarian/
├── config.toml              # Configuration
├── default.db               # Default database (no bucket)
└── buckets/
    ├── psc-4395/
    │   ├── documents.db     # SQLite database
    │   └── generated/       # Generated study materials
    └── cs-101/
        ├── documents.db
        └── generated/
```

## Project Structure

```
src/
├── main.rs           # CLI entry point (headless subcommands + TUI launch)
├── config.rs         # Configuration + provider/transcriber resolution
├── retrieval.rs      # Hybrid search (RRF) + structured citations
├── eval.rs           # Retrieval evaluation harness (recall@k / MRR)
├── search.rs         # Query enhancement + chunk dedup helpers
├── bucket/           # Library/bucket isolation
├── commands/         # Headless command implementations + shared RAG helpers
│   ├── add.rs        # Content ingestion
│   ├── chat.rs       # Chat context builders (shared with the TUI)
│   ├── generate.rs   # Study material generation
│   ├── docs.rs       # Document management
│   └── bucket.rs     # Bucket management
├── embeddings/       # Local embedding generation (FastEmbed)
├── ingest/           # Media ingestion
│   ├── pdf.rs        # PDF extraction
│   ├── text.rs       # Text/Markdown
│   ├── url.rs        # Web scraping & YouTube
│   ├── ocr.rs        # Image OCR (Tesseract)
│   └── chunker.rs    # Structure-aware text chunking
├── llm/              # LLM clients
│   ├── provider.rs   # Pluggable provider layer (Groq/OpenAI/Anthropic/Ollama)
│   ├── groq.rs       # Groq chat API + shared Message type
│   └── whisper.rs    # Whisper transcription
├── tui/              # Full-screen ratatui TUI (panes, services, async worker)
└── storage/          # SQLite storage layer
    ├── db.rs         # Database connection + schema
    ├── documents.rs  # Document CRUD + FTS5
    └── chunks.rs     # Chunk/embedding storage + chunks_fts (FTS5)
```

## Development

```bash
# Run in development mode
cargo run

# Run with debug output
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check for issues
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Troubleshooting

### "No API key configured"
```bash
librarian config
# Or set environment variable:
export GROQ_API_KEY="gsk_..."
```

### "FFmpeg not found" (for video/audio)
```bash
# Arch
sudo pacman -S ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# macOS
brew install ffmpeg
```

### "Tesseract not found" (for images)
```bash
# Arch
sudo pacman -S tesseract tesseract-data-eng

# Ubuntu/Debian
sudo apt install tesseract-ocr tesseract-ocr-eng

# macOS
brew install tesseract
```

### PDF extraction crashes
Some complex PDFs may cause issues. The tool automatically falls back to an alternative extractor (lopdf) when the primary one fails.

### Slow first run
The embedding model (~90MB) is downloaded on first use. Subsequent runs are fast.

### Large video files
Video transcription uploads audio to Groq's Whisper API. Very large files may take time or hit size limits. Consider splitting long recordings.

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/mediaStudy.git

# Create a branch
git checkout -b feature/amazing-feature

# Make changes, then
cargo fmt
cargo clippy -- -D warnings
cargo test

# Commit and push
git commit -m "Add amazing feature"
git push origin feature/amazing-feature

# Open a Pull Request
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Groq](https://groq.com/) - Ultra-fast LLM inference
- [FastEmbed](https://github.com/Anush008/fastembed-rs) - Local embeddings in Rust
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [ratatui](https://github.com/ratatui/ratatui) + [crossterm](https://github.com/crossterm-rs/crossterm) - Full-screen terminal UI
- [colored](https://github.com/colored-rs/colored) - Terminal colors
- [indicatif](https://github.com/console-rs/indicatif) - Progress bars

---

Made with 📚 for students, by students.
