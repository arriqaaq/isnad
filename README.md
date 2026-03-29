# Hadith Explorer

Browse, search, and explore Islamic hadith collections with narrator chain (isnad) visualization and RAG-powered Q&A.

## Setup

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v20+)
- [Ollama](https://ollama.ai/) (for translation and Ask feature)

### Install Ollama

```bash
brew install ollama
ollama pull llama3.2
ollama serve
```

### Build

```bash
# Frontend
cd frontend && npm install && npm run build && cd ..

# Backend
cargo build
```

## Data

The project uses the [Sanadset 650K dataset](https://data.mendeley.com/datasets/5xth87zwb5/4) — 368K+ hadiths from 926 books with pre-parsed narrator chains.

**The dataset is auto-downloaded** during the first ingest if not already present. Or download manually:

```bash
curl -L "https://data.mendeley.com/public-api/zip/5xth87zwb5/download/5" -o data/sanadset.zip
unzip data/sanadset.zip -d data/
# Find the CSV and rename/move to data/sanadset.csv
```

## Ingest

```bash
# List all 926 available books
cargo run -- ingest --list-books

# Ingest the 6 major collections (Kutub al-Sittah) — quick test
cargo run -- ingest --limit 10

# Full ingest with English translation via Ollama
cargo run -- ingest --translate

# Ingest specific books by number (from --list-books)
cargo run -- ingest --books 1,8,13

# Ingest all 926 books
cargo run -- ingest --all

# Use a different Ollama model
cargo run -- ingest --translate --translate-model qwen2.5

# Fresh start
rm -rf db_data && cargo run -- ingest --translate
```

## Run

```bash
cargo run -- serve --port 3000
```

Open http://localhost:3000

## Features

- **Browse** hadiths from 926+ hadith collections
- **Bilingual** Arabic + English (Ollama-translated)
- **Narrator chains** (isnad) — interactive graph from Sanadset data
- **Narrator network** — teachers, students, and hadith connections
- **Search** in both Arabic and English
- **Semantic search** — find hadiths by meaning using vector embeddings
- **Ask** — RAG-powered Q&A grounded in actual hadith text

## Tech Stack

- **Backend**: Rust, Axum, SurrealDB (RocksDB), FastEmbed
- **Frontend**: SvelteKit 2, Svelte 5, Cytoscape.js
- **LLM**: Ollama (local, for translation and RAG)
- **Data**: Sanadset 650K (Mendeley Data)
