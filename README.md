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
- **Semantic search**: Local vector embeddings (all-MiniLM-L6-v2) with cosine similarity
- **Study tools**: Generate study guides, flashcards, quizzes, and summaries - saved to your library
- **Interactive chat**: Ask "The Librarian" questions grounded in your ingested materials
- **Homework help**: Guided problem-solving mode
- **Beautiful CLI**: Polished terminal UI with visual library shelf and status dashboard
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Privacy-first**: Embeddings generated locally, only LLM queries sent to API

## Prerequisites

### Required
- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Groq API Key**: Sign up free at [console.groq.com](https://console.groq.com/)

### Optional (for specific media types)

| Tool | Purpose | Installation |
|------|---------|--------------|
| **FFmpeg** | Video/audio transcription | See [installation](#installing-optional-dependencies) |
| **Tesseract** | Image/screenshot OCR | See [installation](#installing-optional-dependencies) |

## Installation

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

Just run `librarian` with no arguments for the beautiful interactive menu:

```bash
librarian
```

You'll see your library shelf with all your books, a status dashboard, and menu options.

### Commands

```bash
# Content Management
librarian add <path/url>           # Add files, directories, or URLs
librarian list                     # List all documents in current book
librarian search <query>           # Search documents
librarian docs                     # Manage documents (view/delete)

# Study Tools
librarian chat                     # Interactive Q&A with your materials
librarian generate study-guide     # Generate comprehensive study guide
librarian generate flashcards      # Generate flashcards
librarian generate quiz            # Generate practice quiz
librarian generate summary         # Generate summary
librarian generate homework        # Interactive homework help

# Library Organization
librarian bucket create <name>     # Create a new book
librarian bucket list              # List all books
librarian bucket use <name>        # Switch to a book
librarian bucket delete <name>     # Delete a book
librarian library                  # Alias for bucket management

# Configuration
librarian config                   # Configure API key and settings
librarian completions <shell>      # Generate shell completions
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

Configuration is stored at:
- **Linux**: `~/.config/media-study/config.toml`
- **macOS**: `~/Library/Application Support/media-study/config.toml`
- **Windows**: `%APPDATA%\media-study\config.toml`

```toml
groq_api_key = "gsk_..."
default_model = "llama-3.3-70b-versatile"
current_bucket = "psc-4395"
```

Environment variable alternative:
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
│  Response   │◀────│   Groq LLM   │◀────│  Semantic   │
│  (Grounded) │     │   + Context  │     │   Search    │
└─────────────┘     └──────────────┘     └─────────────┘
```

1. **Ingestion**: Extract text from various formats (PDF parsing, Whisper transcription, OCR)
2. **Chunking**: Split into ~1000 char chunks with 200 char overlap for context
3. **Embedding**: Generate 384-dim vectors locally using all-MiniLM-L6-v2
4. **Storage**: SQLite database per book (bucket) with FTS5 full-text search
5. **Search**: Cosine similarity search on query embedding to find relevant chunks
6. **Generation**: Send top relevant chunks as context to Groq LLM for grounded responses

## Models Used

| Purpose | Model | Notes |
|---------|-------|-------|
| Embeddings | all-MiniLM-L6-v2 | Local, ~90MB download on first run |
| Transcription | whisper-large-v3 | Via Groq API, for audio/video |
| Chat/Generation | llama-3.3-70b-versatile | High quality (default) |
| Alternative | llama-3.1-8b-instant | Faster, lower latency |

## Data Storage

Data is stored at:
- **Linux**: `~/.local/share/media-study/`
- **macOS**: `~/Library/Application Support/media-study/`
- **Windows**: `%APPDATA%\media-study\`

Structure:
```
media-study/
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
├── main.rs           # CLI entry point & interactive UI
├── config.rs         # Configuration management
├── bucket/           # Library/bucket isolation
├── commands/         # CLI command implementations
│   ├── add.rs        # Content ingestion
│   ├── chat.rs       # Interactive chat
│   ├── generate.rs   # Study material generation
│   ├── docs.rs       # Document management
│   ├── bucket.rs     # Bucket management
│   └── config.rs     # Settings UI
├── embeddings/       # Local embedding generation (FastEmbed)
├── ingest/           # Media ingestion
│   ├── pdf.rs        # PDF extraction
│   ├── text.rs       # Text/Markdown
│   ├── url.rs        # Web scraping & YouTube
│   ├── ocr.rs        # Image OCR (Tesseract)
│   └── chunker.rs    # Text chunking
├── llm/              # LLM clients
│   ├── groq.rs       # Groq chat API
│   └── whisper.rs    # Groq Whisper transcription
└── storage/          # SQLite storage layer
    ├── db.rs         # Database connection
    ├── documents.rs  # Document CRUD
    └── chunks.rs     # Chunk/embedding storage
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
- [inquire](https://github.com/mikaelmello/inquire) - Beautiful interactive prompts
- [colored](https://github.com/colored-rs/colored) - Terminal colors
- [indicatif](https://github.com/console-rs/indicatif) - Progress bars

---

Made with 📚 for students, by students.
