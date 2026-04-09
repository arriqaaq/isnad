# Training a Custom LLM on Hadith & Quran Data

This guide covers fine-tuning a lightweight, domain-specific LLM on Islamic texts and deploying it through the existing Ollama-based ask loop — with **zero changes** to the Rust backend.

## Architecture Overview

The project uses Ollama for a GraphRAG pipeline (`src/agentic_rag.rs`). The flow is:

```
User question → classify + retrieve (ayahs + hadiths via HNSW, narrator chains via graph)
    → context passed to Ollama /api/chat (streaming)
    → SSE stream to frontend
```

A fine-tuned model simply replaces the default model in Ollama. The `OllamaClient` already supports model override via env var (`OLLAMA_MODEL`), CLI flag (`--model`), or per-request (`{"model": "hadith-scholar"}`).

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

# 3. Generate training data from SemanticHadith KG + Quran
#    Uses semantic_hadith.json (6,786 narrators + 34K hadiths with chains)
#    Ollama optional (for RAG Q&A category only): ollama pull command-r7b-arabic
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

### Data Source: SemanticHadith Knowledge Graph

The training pipeline uses `data/semantic_hadith.json` (SemanticHadith KG V2) as the primary source. No SurrealDB dependency — everything comes from this JSON + `data/quran.csv`.

| Source | Content |
|--------|---------|
| `data/semantic_hadith.json` | 6,786 narrators (with Ibn Hajar reliability grades, generation, biographical data) + 34,011 hadiths (Arabic + English text, full narrator chains) across 6 books |
| `data/quran.csv` | 6,236 Quran verses with Arabic, English, Tafsir Ibn Kathir |

Each narrator in the KG has: name, popularName, generation (tabaqah 1-12), reliabilityGrade (thiqah/saduq/maqbul/majhul/daif/matruk), ibnHajarRank (full Arabic assessment text), teknonym, lineage, residence, death/birth year.

Each hadith has: full narrator chain (as ordered list of narrator IDs), Arabic text, English text, book, referenced Quran verses.

### Training Example Categories (6 categories)

#### Category 1: Hadith Science Terminology (~58 examples)
Hardcoded scholarly definitions from at-Tahhaan's *Tayseer Mustalah al-Hadeeth*. Teaches the model domain vocabulary:
- Chain continuity: muttasil, munqati, mursal, muallaq, mudal
- Breadth classes: mutawatir, mashhur, aziz, gharib
- Narrator reliability: thiqah, saduq, da'if, matruk, majhul, maqbul
- Tabaqat: Sahabi, Tabi'i, Tabi' al-Tabi'in
- Corroboration: mutaba'at, shawahid, i'tibaar
- Structural concepts: Common Link, madar al-isnad, fan-out, bundle coverage, bypass
- Hadith grading: sahih, hasan, da'if, mawdu'

#### Category 2: Narrator Chain Analysis (~300 examples)
From KG narrator data + chains. Given a hadith with its full narrator chain (resolved to names, generation, reliability grades, Ibn Hajar assessments), the model learns to assess chain reliability — identifying strong links, weak links, and overall chain strength.

#### Category 3: Isnad Structural Analysis (~200 examples)
From KG chain patterns. Identifies Common Links (narrators appearing in many chains), analyzes their role in transmission, and evaluates their reliability. Uses actual hadith-count-per-narrator data from the KG.

#### Category 4: Hadith RAG Q&A (~300 examples, uses Ollama)
From KG hadiths with English text. Matches the runtime RAG prompt pattern from `src/rag.rs` — system prompt with hadith context + narrator chains, user question, scholarly answer citing hadith numbers.

#### Category 5: Quran + Tafsir (~200 examples)
From `data/quran.csv`. Uses tafsir text directly as the answer (no Ollama). Long tafsir entries truncated to ~500 words.

#### Category 6: Cross-Domain Hadith↔Quran (~100 examples)
From KG's `quranVerses` field (580 hadiths reference specific Quran verses). Teaches the model to connect hadith narrations with Quranic teachings.

**Target: ~1,150 total examples (~1,035 train + ~115 valid)**

### Running the Script

```bash
# All categories except RAG Q&A (instant, no Ollama needed):
python3 scripts/prepare_training_data.py --rag 0

# Full generation including RAG Q&A (~15 min with 4 Ollama workers):
python3 scripts/prepare_training_data.py --rag 300 --workers 4

# Custom targets:
python3 scripts/prepare_training_data.py --terminology 200 --narrator 300 --structural 200 --rag 300 --tafsir 200 --crossdomain 100
```

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
