# 🎬 Cinephile

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/woodj/cinephile?style=flat-square)](https://github.com/woodj/cinephile/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-blue.svg?style=flat-square)](https://www.rust-lang.org)

**Cinephile** is a high-performance, cross-platform CLI tool for local semantic movie search. It indexes your video collection, enriches it with professional metadata from TMDB, and allows you to search your library using natural language queries powered by local AI.

---

## ✨ Features

- **🔍 Semantic Search:** Find movies by plot details, vibes, or themes (e.g., "a mind-bending space thriller") using local vector embeddings.
- **🤖 Local AI Integration:** Uses [Ollama](https://ollama.com) for query intent classification and [ONNX Runtime](https://onnxruntime.ai) for local embedding generation.
- **📽️ Metadata Enrichment:** Automatically fetches plot summaries, directors, cast members, and release years from TMDB.
- **⚡ High Performance:** Built with Rust, utilizing `SurrealDB` for fast data retrieval and `tokio` for asynchronous I/O.
- **🚀 Cross-Platform:** Single binary executables for Windows, macOS, and Linux.

---

## 🛠️ Prerequisites

Before you begin, ensure you have the following:

1.  **[Ollama](https://ollama.com):** Download and run Ollama on your machine.
    - Pull the required model: `ollama pull llama3.2`
2.  **TMDB API Key:** Create an account at [TheMovieDB](https://www.themoviedb.org/settings/api) to generate a free API key.

---

## 🚀 Installation

### Download Binaries (Recommended)
Download the latest pre-compiled binary for your system from the [Releases](https://github.com/woodj/cinephile/releases) page.

### Build from Source
If you have the Rust toolchain installed:
```bash
git clone https://github.com/woodj/cinephile.git
cd cinephile
cargo build --release
```
The binary will be located at `./target/release/cinephile_cli`.

---

## 📖 Usage Guide

### 1. Configuration
Set your TMDB API key to enable metadata enrichment:
```bash
cinephile config --set-tmdb-key YOUR_TMDB_API_KEY
```

### 2. Add Movie Directories
Add one or more directories containing your video files:
```bash
cinephile add ~/Movies/Sci-Fi
cinephile add "D:\Videos\Movies"
```

### 3. Index Your Collection
Scan your directories, fetch metadata, and generate semantic embeddings. This step requires an internet connection for TMDB and Ollama running locally.
```bash
cinephile index
```

### 4. Search
Perform a semantic search using natural language:
```bash
# Semantic search
cinephile search "a heist movie with a twist"

# Search and play the top result automatically
cinephile search "interstellar" --play
```

---

## 🏗️ Architecture

Cinephile is split into two main crates:
- **`cinephile_core`**: The library containing the Crawler, Enricher (TMDB + Ollama), Embedder (ONNX), and Store (SurrealDB).
- **`cinephile_cli`**: The user-facing command-line interface.

---

## ⚖️ License

Distributed under the MIT License. See `LICENSE` for more information.

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
