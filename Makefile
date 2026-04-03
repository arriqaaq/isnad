.PHONY: build frontend backend server dev stop ingest ingest-test ingest-full list-books quran-prepare quran-ingest quran-hadith-refs quran-morphology quran-similar quran-manuscripts quran quran-full analyze analyze-bio analyze-families analyze-transmission pipeline-test pipeline-full clean

# SurrealDB HNSW index traversal needs extra stack space
export RUST_MIN_STACK=8388608

# Build everything
build: backend frontend

# Build frontend
frontend:
	cd frontend && npm install && npm run build

# Build backend
backend:
	cargo build

# Start server (foreground) with qwen3:8b
server: build
	cargo run -- serve --port 3000 --ollama-model qwen3:8b

# Build everything and start server in background with qwen3:8b
dev: build
	@echo "Starting server on http://localhost:3000..."
	@cargo run -- serve --port 3000 --ollama-model qwen3:8b &
	@sleep 2
	@echo "Server running at http://localhost:3000 (use 'make stop' to shut down)"

# Stop background server
stop:
	@pkill -f "target/debug/hadith serve" 2>/dev/null && echo "Server stopped" || echo "No server running"

# List available books in dataset
list-books:
	cargo run -- ingest --list-books

# Quick test ingest (5 per book, with translation via qwen3:8b)
ingest-test:
	rm -rf db_data
	cargo run -- ingest --limit 5 --translate --translate-model qwen3:8b

# Full ingest of 6 major books with translation via qwen3:8b
ingest-full:
	rm -rf db_data
	cargo run -- ingest --translate --translate-model qwen3:8b

# Ingest with human English translations (no Ollama needed)
ingest:
	rm -rf db_data
	cargo run -- ingest

# === Quran ingestion ===

VENV := .venv
VENV_PYTHON := $(VENV)/bin/python3
VENV_PIP := $(VENV)/bin/pip

# Create virtual environment and install Python dependencies
$(VENV_PYTHON):
	python3 -m venv $(VENV)
	$(VENV_PIP) install --upgrade pip
	$(VENV_PIP) install datasets pandas

# Prepare Quran data (download Tanzil + Tafsir Ibn Kathir, merge into CSV)
quran-prepare: $(VENV_PYTHON)
	$(VENV_PYTHON) scripts/prepare_quran_data.py

# Ingest Quran into SurrealDB (requires data/quran.csv from quran-prepare)
quran-ingest:
	cargo run -- ingest-quran

# Ingest Quran→Hadith reference mappings from Quran.com (requires ingest + quran-ingest first)
quran-hadith-refs:
	cargo run -- ingest-quran-hadith-refs

# Download morphology data (if not present)
data/quran-morphology.txt:
	curl -sL https://raw.githubusercontent.com/mustafa0x/quran-morphology/master/quran-morphology.txt -o data/quran-morphology.txt

# 1. Ingest word morphology (must run before quran-similar so phrase text can be extracted)
#    Requires: data/quran-morphology.txt (auto-downloaded)
#    Optional: data/colored-english-wbw-translation.json (download "Colored English wbw translation" JSON from qul.tarteel.ai/resources/translation)
quran-morphology:
	cargo run -- ingest-morphology

# 2. Ingest mutashabihat + similar ayahs from QUL JSON (must run after morphology)
#    Requires: data/qul/phrases.json + data/qul/matching-ayah.json (download from qul.tarteel.ai/resources)
quran-similar:
	cargo run -- ingest-quran-similar --qul-dir data/qul

# 3. Clone Corpus Coranicum TEI (if not present) and ingest manuscripts + variant readings
quran-manuscripts:
	@test -d data/corpus-coranicum-tei || git clone https://github.com/telota/corpus-coranicum-tei.git data/corpus-coranicum-tei
	cargo run -- ingest-manuscripts --tei-dir data/corpus-coranicum-tei

# Base Quran pipeline: prepare data + ingest ayahs + hadith refs
quran: quran-prepare quran-ingest quran-hadith-refs

# Full Quran pipeline (ordered: base → morphology → similar → manuscripts)
quran-full: quran data/quran-morphology.txt quran-morphology quran-similar quran-manuscripts

# === Analyze phase (runs on already-ingested data) ===

# Enrich narrators with AR-Sanad biographical data (auto-downloads dataset)
analyze-bio:
	cargo run -- analyze --narrator-bio data/ar_sanad_narrators.csv

# Compute hadith families from embedding similarity
analyze-families:
	cargo run -- analyze --families

# Run all analysis: narrator bios + families
analyze:
	cargo run -- analyze --narrator-bio data/ar_sanad_narrators.csv --families

# Run transmission integrity analysis (CL/PCL + structural falsifiability tests)
analyze-transmission:
	cargo run -- analyze --juynboll

# Full pipeline: ingest 100 per book + all analysis
pipeline-test:
	rm -rf db_data
	cargo run -- ingest --limit 100
	cargo run -- analyze --narrator-bio data/ar_sanad_narrators.csv --families
	cargo run -- analyze --juynboll

# Full pipeline with all data from 6 major books
pipeline-full:
	rm -rf db_data
	cargo run -- ingest
	$(MAKE) quran
	cargo run -- analyze --narrator-bio data/ar_sanad_narrators.csv --families
	cargo run -- analyze --juynboll

# Clean all generated data
clean:
	rm -rf db_data target frontend/build frontend/node_modules $(VENV)
