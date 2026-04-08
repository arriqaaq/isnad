# Training a Custom LLM on Hadith & Quran Data

This guide covers fine-tuning a lightweight, domain-specific LLM on Islamic texts and deploying it through the existing Ollama-based ask loop — with **zero changes** to the Rust backend.

## Architecture Overview

The project already uses Ollama for the ask loop (`src/rag.rs`). The flow is:

```
User question → FastEmbed (1024-dim) → HNSW vector search → top 6 hadiths
    → Graph traversal (narrator chains) → Build context
    → POST to Ollama /api/chat (streaming)
    → SSE stream to frontend
```

A fine-tuned model simply replaces the default `llama3.2` in Ollama. The `OllamaClient` already supports model override via env var (`OLLAMA_MODEL`), CLI flag (`--model`), or per-request (`{"model": "hadith-scholar"}`).

---

## Quick Start

Run these commands from the project root to go from raw data to a working model:

```bash
# 1. Create virtualenv and install dependencies
python3 -m venv .venv-train
source .venv-train/bin/activate
pip install mlx-lm datasets

# 2. Generate Quran CSV (loads QUL QPC Hafs + QUL Tafsir Ibn Kathir)
python3 scripts/prepare_quran_data.py

# 3. Generate training data from hadith/Quran sources
#    Requires: Ollama running with llama3.2 (ollama pull llama3.2 && ollama serve)
#    Parses sanadset.csv (30K hadiths) → enriches with translations → generates ~1,400 Q&A examples
python3 scripts/prepare_training_data.py

# 4. Fine-tune Phi-4-mini with LoRA (~20-30 min on Apple Silicon)
mlx_lm.lora \
  --model mlx-community/Phi-4-mini-instruct-4bit \
  --train \
  --data data/ \
  --iters 1000 \
  --batch-size 4 \
  --lora-layers 16 \
  --adapter-path models/hadith-scholar-lora

# 5. Fuse LoRA adapter into base model (--de-quantize required for GGUF conversion)
mlx_lm.fuse \
  --model mlx-community/Phi-4-mini-instruct-4bit \
  --adapter-path models/hadith-scholar-lora \
  --save-path models/hadith-scholar-fused \
  --de-quantize

# 6. Convert to GGUF
git clone https://github.com/ggml-org/llama.cpp.git
pip install -r llama.cpp/requirements.txt
python3 llama.cpp/convert_hf_to_gguf.py models/hadith-scholar-fused \
  --outfile models/hadith-scholar.gguf \
  --outtype q8_0

# 7. Quantize to Q4_K_M (~2GB file)
cd llama.cpp && cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build --target llama-quantize -j && cd ..
llama.cpp/build/bin/llama-quantize models/hadith-scholar.gguf models/hadith-scholar-q4km.gguf Q4_K_M

# 8. Register with Ollama and run
deactivate
ollama create hadith-scholar -f models/Modelfile
ollama run hadith-scholar "What is the significance of intentions in Islam?"

# 9. Use with the web app
OLLAMA_MODEL=hadith-scholar make server
```

Alternatively, `scripts/train_mlx.sh` wraps steps 1 and 4-7 into a single script:

```bash
python3 scripts/prepare_quran_data.py          # step 2
python3 scripts/prepare_training_data.py       # step 3
bash scripts/train_mlx.sh                      # steps 1, 4-7
ollama create hadith-scholar -f models/Modelfile  # step 8
```

---

## How It Works

The sections below explain each step in detail.

## Step 0: Prerequisites

Before preparing training data, generate the Quran CSV if it doesn't already exist:

```bash
# quran.csv must be generated first (loads QUL QPC Hafs + QUL Tafsir Ibn Kathir)
python3 scripts/prepare_quran_data.py
# Verify: data/quran.csv should have 6,236 rows
```

---

## Step 1: Prepare Training Data

### Data Hierarchy

**Sanadset is the source of truth.** After ingestion, SurrealDB contains the fully joined data: hadiths with Arabic text, English translations (merged from sunnah.com), narrator chains (graph edges), and narrator metadata. The other CSVs enrich sanadset during ingest.

| Source | Role | Content |
|--------|------|---------|
| `data/sanadset.csv` (368K rows) | **Primary** — ingested into SurrealDB | Arabic text, matn, narrator chains (isnad) |
| `data/translations/*.csv` (33,738 rows) | Enrichment — merged during ingest | English translations, chapter titles, grades |
| `data/ar_sanad_narrators.csv` (18K+ rows) | Enrichment — merged during ingest | Narrator reliability ratings, bios |
| `data/quran.csv` (6,236 rows) | Separate corpus | Quran verses, translations, Tafsir Ibn Kathir |

### Data Extraction: Parse Raw CSVs Directly

The training pipeline works from raw CSV files — no SurrealDB dependency. This keeps the pipeline standalone and reproducible.

**Sanadset parsing** (`data/sanadset.csv`):
- Column 0 (`Hadith`): Full Arabic text with XML tags — strip `<SANAD>`, `<MATN>`, `<NAR>` tags
- Column 1 (`Book`): Arabic book name (e.g., "صحيح البخاري")
- Column 2 (`Num_hadith`): Hadith number (global ID)
- Column 3 (`Matn`): Content without isnad (the actual teaching)
- Column 4 (`Sanad`): Narrator chain as Python list string: `['narrator1', 'narrator2', ...]`
- Column 5 (`Sanad_Length`): Chain length

**Enrichment from translations** (`data/translations/*.csv`):
- Match sanadset hadiths to translations by book + hadith number
- Adds: English text, chapter titles, grades

**Enrichment from narrator bios** (`data/ar_sanad_narrators.csv`, optional):
- Match narrator names from sanadset's Sanad column
- Adds: reliability ratings, generation, biographical info

The script builds a unified hadith record: `(hadith_number, book, text_ar, matn, sanad_chain[], text_en?, chapter_title?, narrator_reliability?)`

### Why Training Data Must Match the Runtime Pattern

At runtime, the ask loop in `src/rag.rs` sends the LLM a **specific prompt pattern**:

```
System: You are a knowledgeable Islamic scholar assistant...
        Answer questions using ONLY the hadiths provided below as context.
        Always cite the hadith number when referencing a hadith.
        When relevant, mention the chain of narration (isnad)...

        ## Relevant Hadiths:

        Hadith #1234 — Abu Hurayra
        Chain of narration: Abu Hurayra → Ibn Shihab → Malik
        [hadith English text]

        Hadith #5678 — Aisha
        Chain of narration: Aisha → Urwa → Az-Zuhri
        [hadith English text]

User:   What does Islam say about intentions?
```

**The training data must mirror this exact pattern.** The model's job is to reason over provided hadith context — not to recall hadiths from memory. Training with standalone Q&A would teach the wrong task.

### Training Data Format

ChatML messages JSONL (compatible with both mlx-lm and SFTTrainer):

```jsonl
{"messages": [{"role": "system", "content": "You are a knowledgeable Islamic scholar assistant...\n\n## Relevant Hadiths:\n\nHadith #1 — Umar bin Al-Khattab\nChain of narration: Al-Humaydi → Sufyan → Yahya bin Said\nNarrated Umar bin Al-Khattab: I heard Allah's Messenger saying, \"The reward of deeds depends upon the intentions...\"\n\n"}, {"role": "user", "content": "What is the significance of intentions in Islam?"}, {"role": "assistant", "content": "According to Hadith #1, narrated by Umar bin Al-Khattab through the chain Al-Humaydi → Sufyan → Yahya bin Said, the Prophet (ﷺ) said: \"The reward of deeds depends upon the intentions.\" This foundational hadith establishes that every action in Islam is judged by its underlying intention (niyyah). The chain includes Al-Humaydi (thiqah/trustworthy) and Sufyan ibn Uyaynah (thiqah), making this a strong chain..."}]}
```

### Training Example Categories

The script generates examples in **3 categories**, each matching a real usage pattern:

#### Category 1: Hadith RAG Q&A (~800 examples)
Parses sanadset CSV, enriches with translations. For each example:
1. Samples 2-4 hadiths from the same book (simulating RAG retrieval)
2. Extracts the narrator chain from sanadset's `Sanad` column (parsed from Python list string)
3. Formats them with the exact system prompt from `rag.rs`, including chain of narration
4. Generates a question about the topic
5. Generates an answer that cites hadith numbers, discusses the isnad, and explains the teaching

**Question templates** (varied):
- "What does {Book} say about {topic}?"
- "Explain the teaching in these hadiths"
- "What guidance did the Prophet give regarding {topic}?"
- "Assess the reliability of these narrations on {topic}"

**Answer generation**: Use Ollama (llama3.2) to generate answers given the full hadith context (text + chains + reliability ratings), then post-process to ensure citations are present.

#### Category 2: Quran Tafsir Q&A (~400 examples)
Uses `data/quran.csv`. For each verse with tafsir:
1. Provides the verse (Arabic + English) as context
2. Question: "What is the meaning of Surah {name}, Ayah {num}?"
3. Answer: Tafsir explanation

**Note on tafsir length**: Some tafsir entries are very long (2000+ words for key verses like Al-Baqarah). Since training uses a fixed sequence length (2048 tokens default), examples exceeding this are silently truncated. Two options:
- **Option A**: Increase `--max-seq-length 4096` during training (uses more memory but preserves full tafsir)
- **Option B**: For very long tafsir entries, use Ollama to generate a concise scholarly summary preserving key references, keeping within the default sequence budget

#### Category 3: Narrator Chain Analysis (~200 examples)
Uses sanadset narrator chains, optionally enriched with ar_sanad_narrators reliability data. Given a hadith:
1. Extracts the narrator chain from sanadset's `Sanad` column, cross-references with `ar_sanad_narrators.csv` for reliability ratings
2. Question: "How reliable is the chain of narration for this hadith?"
3. Answer: Assesses each narrator's reliability rating (thiqah, saduq, da'if, etc.), notes any weak links, overall chain strength

### Data Generation Script

Create `scripts/prepare_training_data.py`:

```python
#!/usr/bin/env python3
"""
Generate instruction-tuning data for hadith-scholar model.

Parses raw CSV files directly (no SurrealDB dependency):
  - data/sanadset.csv        (source of truth: Arabic text + narrator chains)
  - data/translations/*.csv  (enrichment: English translations + chapter titles)
  - data/ar_sanad_narrators.csv (enrichment: narrator reliability ratings)
  - data/quran.csv           (Quran verses + Tafsir Ibn Kathir)

Uses Ollama to generate Q&A pairs formatted to match the RAG prompt
pattern in src/rag.rs.

Prerequisites:
  - Ollama running locally with llama3.2
  - data/quran.csv (run: python3 scripts/prepare_quran_data.py)

Output:
  - data/train.jsonl  (~1400 examples)
  - data/valid.jsonl   (~150 examples)
"""
```

The script should:
1. Parse sanadset CSV → extract hadith text, matn, narrator chains (strip XML tags, parse Sanad list)
2. Parse translation CSVs → build lookup by book+hadith_number for English text + chapter titles
3. Optionally parse ar_sanad_narrators CSV → build narrator name → reliability rating lookup
4. Join: enrich sanadset hadiths with English translations and narrator reliability
5. Group hadiths by book for RAG simulation
6. Parse `data/quran.csv` for tafsir examples
4. For each training example, call Ollama to generate the assistant response
5. Format as ChatML messages JSONL matching the `rag.rs` system prompt
6. Shuffle and split 90/10 into `train.jsonl` / `valid.jsonl`
7. Validate: ensure every example has citations, reasonable length

**Target: ~1,400 training + ~150 validation examples.**

### Data Quality Rules

- Every assistant response MUST cite at least one hadith number (e.g., "Hadith #1234")
- Every response mentioning a hadith MUST reference its narrator and chain
- Narrator reliability assessments must use the actual ratings from the DB (not hallucinated)
- Tafsir responses must reference the surah and ayah number
- Answers should be 100-500 words (matching the concise style in the system prompt)
- Arabic terms should include English in parentheses: "isnad (chain of narration)"
- No hallucinated hadith numbers — only use numbers present in the context

---

## Step 2: Fine-Tune with LoRA

### Path A: MLX on Mac (Recommended for Local Training)

MLX is Apple's native ML framework, optimized for unified memory on Apple Silicon. This is the correct choice for training on a MacBook.

> **Why not Unsloth?** Unsloth requires Triton/NVIDIA GPU — it does **not** work on Apple Silicon.

#### Base Model

**`mlx-community/Phi-4-mini-instruct-4bit`** (3.8B parameters)
- Uses ChatML format (matches our training data format)
- 128K context window
- Strong multilingual (Arabic + English) support
- ~2GB memory footprint in 4-bit quantization

Alternative lighter models:
- `CohereForAI/c4ai-command-r7b-arabic` — 7B params, Arabic-tuned, strong multilingual
- `mlx-community/SmolLM2-1.7B-Instruct-4bit` — 1.7B params, ultra-light (fits in 8GB)

#### Setup Virtual Environment

```bash
# Create and activate a dedicated virtualenv for training
python3 -m venv .venv-train
source .venv-train/bin/activate

# Install dependencies
pip install mlx-lm datasets
```

#### Train

```bash
# Ensure virtualenv is active
source .venv-train/bin/activate

mlx_lm.lora \
  --model mlx-community/Phi-4-mini-instruct-4bit \
  --train \
  --data data/ \
  --iters 1000 \
  --batch-size 4 \
  --lora-layers 16 \
  --adapter-path models/hadith-scholar-lora
```

The `--data data/` flag expects `data/train.jsonl` and optionally `data/valid.jsonl`.

**Training time:** ~20-30 minutes on MacBook M-series with 16GB unified memory.

#### Fuse LoRA Weights

After training, merge the LoRA adapter back into the base model. The `--de-quantize` flag is **required** — 4-bit quantized weights cannot be directly converted to GGUF, so fuse outputs full-precision HuggingFace safetensors:

```bash
source .venv-train/bin/activate

mlx_lm.fuse \
  --model mlx-community/Phi-4-mini-instruct-4bit \
  --adapter-path models/hadith-scholar-lora \
  --save-path models/hadith-scholar-fused \
  --de-quantize
```

#### Test Locally (Optional)

```bash
source .venv-train/bin/activate

mlx_lm.generate \
  --model models/hadith-scholar-fused \
  --prompt "What is the significance of the isnad in hadith verification?"
```

### Path B: Unsloth on Google Colab (Free GPU Alternative)

Use this if your Mac has <16GB RAM or you want faster training on a free NVIDIA T4 GPU.

```python
# Run in Google Colab
!pip install unsloth

from unsloth import FastLanguageModel
from trl import SFTTrainer
from datasets import load_dataset

# Load Phi-4-mini with 4-bit quantization
model, tokenizer = FastLanguageModel.from_pretrained(
    "unsloth/Phi-4-mini-instruct",
    max_seq_length=2048,
    load_in_4bit=True,
)

# Add LoRA adapters
model = FastLanguageModel.get_peft_model(
    model,
    r=16,
    lora_alpha=16,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
    lora_dropout=0,
)

# Load training data (upload train.jsonl to Colab first)
dataset = load_dataset("json", data_files="train.jsonl", split="train")

# Train (2 epochs to avoid overfitting on small dataset)
trainer = SFTTrainer(
    model=model,
    train_dataset=dataset,
    max_seq_length=2048,
    num_train_epochs=2,
    per_device_train_batch_size=4,
    learning_rate=2e-4,
    output_dir="hadith-scholar",
)
trainer.train()

# Export directly to GGUF
model.save_pretrained_gguf(
    "hadith-scholar-gguf",
    tokenizer,
    quantization_method="q4_k_m"
)
```

Download the resulting GGUF file from Colab to your local `models/` directory.

---

## Step 3: Convert to GGUF

GGUF is the standard format for efficient local inference via llama.cpp and Ollama.

### From MLX (Path A)

```bash
# Activate the training virtualenv (has the required Python deps)
source .venv-train/bin/activate

# Clone llama.cpp (one-time setup)
git clone https://github.com/ggml-org/llama.cpp.git
cd llama.cpp
pip install -r requirements.txt

# Convert fused HuggingFace model to GGUF
python convert_hf_to_gguf.py ../models/hadith-scholar-fused \
  --outfile ../models/hadith-scholar.gguf \
  --outtype q8_0

# Quantize to Q4_K_M for smaller file size
./llama-quantize ../models/hadith-scholar.gguf \
  ../models/hadith-scholar-q4km.gguf Q4_K_M
```

### From Unsloth (Path B)

Already exported as GGUF in Step 2 — just download from Colab.

### Quantization Options

| Level | File Size (3.8B) | Quality Retention | Use Case |
|-------|------------------|-------------------|----------|
| Q4_K_M | ~2.0 GB | ~92% | **Recommended** — best balance |
| Q5_K_M | ~2.3 GB | ~95% | Higher quality, slightly larger |
| Q8_0 | ~3.8 GB | ~99% | Maximum quality |

---

## Step 4: Deploy via Ollama

### Create Modelfile

Create `models/Modelfile`:

```dockerfile
FROM ./hadith-scholar-q4km.gguf

PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER num_ctx 4096

SYSTEM "You are a knowledgeable Islamic scholar assistant specializing in hadith sciences and Quranic exegesis."
```

### Register and Test

```bash
# Register with Ollama
ollama create hadith-scholar -f models/Modelfile

# Test interactively
ollama run hadith-scholar "Explain the significance of Hadith #1 in Sahih al-Bukhari"

# Verify streaming API (same endpoint the Rust backend uses)
curl http://localhost:11434/api/chat -d '{
  "model": "hadith-scholar",
  "messages": [{"role": "user", "content": "What is isnad?"}],
  "stream": true
}'
```

---

## Step 5: Use in the Ask Loop

No code changes needed. The existing `OllamaClient` in `src/rag.rs` already supports custom models:

```bash
# Option A: Environment variable
OLLAMA_MODEL=hadith-scholar make server

# Option B: CLI flag
cargo run -- serve --model hadith-scholar

# Option C: Per-request (frontend supports model selection)
# POST /api/ask {"question": "...", "model": "hadith-scholar"}
```

---

## Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Training (MLX, local) | MacBook 8GB (use SmolLM2 1.7B) | MacBook 16GB+ (use Phi-4-mini 3.8B) |
| Training (Colab) | Any browser | Free T4 GPU |
| Inference (Ollama) | 4GB RAM | 8GB+ RAM |

---

## Troubleshooting

### MLX training runs out of memory
- Reduce `--batch-size` to 1 or 2
- Use a smaller model: `mlx-community/SmolLM2-1.7B-Instruct-4bit`
- Reduce `--lora-layers` to 8

### Unsloth save_pretrained_gguf fails
- Known issue on some configurations. Reduce `maximum_memory_usage` or use llama.cpp conversion manually
- Alternative: save as HuggingFace format first, then convert with llama.cpp

### Ollama model gives poor responses
- Check training data quality — garbage in, garbage out
- Increase training iterations (try 2000 instead of 1000)
- Try Q5_K_M quantization instead of Q4_K_M for better quality
- The RAG context from `rag.rs` already provides relevant hadiths — the model mainly needs to reason about them well

### Model doesn't understand Arabic
- Phi-4-mini has multilingual support but may need more Arabic training examples
- Consider Command-R7B-Arabic (`CohereForAI/c4ai-command-r7b-arabic`) which has native Arabic support
- Ensure training data includes Arabic terms with transliterations
