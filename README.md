<p align="center">
  <img src="img/ilm.svg" alt="Ilm — Islamic Knowledge Platform" width="700">
</p>

<p align="center">
  <strong>Search the Quran & Sunnah. <em>Deeply.</em></strong><br>
  A semantic search platform for Islamic scholarship — Quran with tafsir, 368K+ hadiths with narrator chains, and interactive isnad graphs.
</p>

---

> **[Methodology & Algorithms](docs/METHODOLOGY.md)** — Mustalah al-hadith isnad analysis &nbsp;|&nbsp; **[Data Sources](docs/DATA_SOURCES.md)** — Dataset documentation
>
> See also Barmaver's [*Dismantling Orientalist Narratives*](https://www.academia.edu/143038577/Dismantling_Orientalist_Narratives_A_Critique_of_Orientalists_Approach_to_Hadith_with_special_focus_on_Juynboll) (2025, free on Academia.edu).

## Architecture

<p align="center">
  <img src="img/architecture.svg" alt="Architecture overview" width="700">
</p>

Rust backend serving a SvelteKit SPA, with SurrealDB as a unified graph + vector + full-text database. Embeddings via FastEmbed, LLM via local Ollama.

## Features

- **Quran Reader** — 114 surahs with Tajweed Arabic, Sahih International translation, expandable Tafsir Ibn Kathir per ayah
- **Hadith Explorer** — 368K+ hadiths from 926 books across the 6 canonical collections
- **Narrator Networks** — 18K+ narrators with interactive Cytoscape.js graph visualization, Ibn Hajar reliability grades
- **Hybrid Search** — BM25 full-text + 1024-dim semantic vectors fused with Reciprocal Rank Fusion
- **Ask AI (GraphRAG)** — Natural language Q&A grounded in Quran/Hadith via local Ollama, with isnad-aware context
- **Early Manuscripts** — Per-ayah high-resolution manuscript images from Corpus Coranicum (Berlin-Brandenburg Academy), viewable with zoom
- **Isnad Analysis** — Hadith family clustering, mustalah-based chain grading (sahih/hasan/da'eef), transmission breadth (mutawatir/mashhur/aziz/gharib), corroboration detection (mutaba'at/shawahid), word-level matn diffing
- **Personal Study Notes** — Annotate any ayah or hadith, collect evidence by topic with @mentions that embed Quran verses and hadiths inline, tag-based organization, color-coded highlights, and full-text search across your notes

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v20+)
- [Ollama](https://ollama.ai/) — `ollama pull command-r7b-arabic && ollama serve`

### Build & Run

```bash
make build                    # build backend + frontend
make ingest                   # ingest 6 major books (auto-downloads data)
make analyze                  # enrich narrators + cluster families
make quran                    # download + ingest Quran + tafsir
make dev                      # start server at localhost:3000
```

> **Note:** SurrealDB's HNSW vector index requires extra stack space. When running `cargo run` directly (outside of `make`), set `RUST_MIN_STACK=8388608`. The Makefile handles this automatically.

## Data Sources

| Dataset | Records | Content |
|---|---|---|
| [SemanticHadith KG V2](https://github.com/A-Kamran/SemanticHadith-V2) | 34K hadiths | Knowledge graph with narrator chains across 6 canonical collections |
| [Sunnah.com](https://huggingface.co/datasets/meeAtif/hadith_datasets) | 33K translations | Human English for 6 canonical collections |
| [QUL (Tarteel)](https://qul.tarteel.ai/) | 6,236 ayahs | QPC Hafs Arabic + Sahih International English |
| [Tafsir Ibn Kathir](https://qul.tarteel.ai/resources/tafsir/35) | 6,236 ayahs | Classical exegesis in English (HTML) |
| [AR-Sanad](https://github.com/somaia02/Narrator-Disambiguation) | 18K narrators | Ibn Hajar reliability classifications (Taqrib al-Tahdhib) |

All datasets are auto-downloaded on first run. See [DATA_SOURCES.md](docs/DATA_SOURCES.md) for details.

## Ingest Pipeline

<p align="center">
  <img src="img/ingest-pipeline.svg" alt="Ingest pipeline" width="700">
</p>

Parses the SemanticHadith KG, builds the narrator graph, generates embeddings, and merges human English translations from sunnah.com. Use `--translate` to fill gaps with Ollama.

## Search

<p align="center">
  <img src="img/search-flow.svg" alt="Search flow" width="700">
</p>

Three modes: **Hybrid** (default — BM25 + vector via Reciprocal Rank Fusion), **Text** (substring match), and **Semantic** (pure vector similarity). Works across both Arabic and English text.

## Ask (GraphRAG)

<p align="center">
  <img src="img/graphrag-flow.svg" alt="GraphRAG flow" width="700">
</p>

Ask questions in natural language. The system retrieves the 6 most relevant hadiths via vector search, traverses the narrator graph to reconstruct each isnad (chain of narration), then streams an answer from a local LLM grounded in the retrieved context. Responses include narrator chain citations.

## Graph Model

<p align="center">
  <img src="img/graph-model.svg" alt="Database graph model" width="700">
</p>

SurrealDB stores narrators, hadiths, and books as documents connected by `heard_from`, `narrates`, and `belongs_to` graph edges — enabling isnad reconstruction and network analysis.

## Early Manuscripts

<p align="center">
  <img src="img/manuscript-sample.jpg" alt="Early Quranic manuscript — Berlin, Wetzstein II 1913" width="700">
  <br><em>Berlin, Staatsbibliothek: Wetzstein II 1913 — Surah 2:238</em>
</p>

Per-ayah manuscript images from [Corpus Coranicum](https://corpuscoranicum.de/) (Berlin-Brandenburg Academy of Sciences). Click "Manuscripts" on any ayah to view high-resolution scans of early Quranic manuscripts — fetched live from the Corpus Coranicum API.

## Personal Study Notes

<p align="center">
  <img src="img/notes.svg" alt="Personal Study Notes" width="700">
</p>

Annotate any ayah or hadith with personal notes. Collect evidence by topic using @mentions that embed Quran verses and hadiths inline as rich cards. Organize with tags and color-coded highlights. Notes are stored in a separate `user_note` table — safely deletable without impacting ingested data.

- **@Mentions** — type `@2:255` to embed a Quran ayah, `@im_1` for a hadith, or search narrators by name
- **Topic Collections** — save ayahs and hadiths from anywhere into named study notes via the "Save" button
- **Tags & Search** — tag notes for organization, search across all notes by content or tag
- **Color Highlights** — 5 color options (yellow, green, blue, pink, purple) for visual categorization
- **Rich Embeds** — embedded references show the actual Arabic text and translation inline

## Training Pipeline

<p align="center">
  <img src="img/training-pipeline.svg" alt="Training pipeline" width="700">
</p>

Fine-tune a domain-specific LLM on hadith and Quran data, then deploy it through the existing Ollama-based ask loop with zero backend changes. The pipeline generates ~1,400 ChatML Q&A pairs matching the exact RAG prompt pattern from `rag.rs`, fine-tunes via LoRA (MLX locally or Unsloth on Colab), and exports to GGUF for Ollama. See [TRAINING.md](docs/TRAINING.md) for the full guide.

## Tech Stack

| Layer | Technology | Purpose |
|---|---|---|
| Backend | Rust, Axum | HTTP server, JSON API |
| Database | SurrealDB (SurrealKV) | Graph + HNSW vectors + BM25 full-text |
| Embeddings | FastEmbed (bge-m3) | 1024-dim semantic vectors |
| Frontend | SvelteKit 2, Svelte 5 | SPA served as static files |
| Graph Viz | Cytoscape.js | Narrator network visualization |
| LLM | Ollama (local) | Translation fallback + RAG Q&A |

## Contributing

```bash
git clone <repo> && cd hadith
make build
cargo run -- ingest --limit 5 --translate   # quick test data
cd frontend && npm run dev                   # hot reload at :5173
```

See [METHODOLOGY.md](docs/METHODOLOGY.md) for the scholarly framework and [DATA_SOURCES.md](docs/DATA_SOURCES.md) for dataset documentation.
