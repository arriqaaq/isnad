.PHONY: build frontend backend server dev stop semantic-download semantic-extract semantic-verify semantic-setup ingest ingest-test ingest-full hadith-full hadith-ingest sanadset-download quran-prepare quran-prepare-deps quran-ingest quran-hadith-refs quran-morphology quran-similar quran quran-full quran-check analyze analyze-families analyze-transmission pipeline-check pipeline-test pipeline-full clean

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

# Start server (foreground) with command-r7b-arabic
server: build
	cargo run -- serve --port 3000 --ollama-model command-r7b-arabic

# Build everything and start server in background with command-r7b-arabic
dev: build
	@echo "Starting server on http://localhost:3000..."
	@cargo run -- serve --port 3000 --ollama-model command-r7b-arabic &
	@sleep 2
	@echo "Server running at http://localhost:3000 (use 'make stop' to shut down)"

# Stop background server
stop:
	@pkill -f "target/debug/hadith serve" 2>/dev/null && echo "Server stopped" || echo "No server running"

# === SemanticHadith KG data preparation (one-time) ===

# Download SemanticHadith KG V2 TTL (one-time, ~27MB compressed)
semantic-download:
	@mkdir -p /tmp
	curl -L -o /tmp/SemanticHadithKGV2.ttl.zip \
		https://github.com/A-Kamran/SemanticHadith-V2/raw/main/SemanticHadithKGV2.ttl.zip
	cd /tmp && unzip -o SemanticHadithKGV2.ttl.zip
	@echo "✓ SemanticHadith TTL extracted to /tmp/SemanticHadithKGV2.ttl"

# Extract TTL to JSON (one-time, requires rdflib: pip install rdflib)
semantic-extract:
	python3 scripts/build_semantic_data.py

# Verify extracted data
semantic-verify:
	python3 scripts/verify_semantic_data.py

# Full SemanticHadith setup (download + extract + verify)
semantic-setup: semantic-download semantic-extract semantic-verify

# === Hadith ingestion ===

# Quick test ingest (5 per book)
ingest-test:
	cargo run -- ingest --limit 5

# Full hadith pipeline: ingest all 6 books + families + transmission analysis
hadith-full:
	@echo ""
	@echo "═══════════════════════════════════════"
	@echo "  Step 1/3: Ingesting hadith data"
	@echo "═══════════════════════════════════════"
	cargo run -- ingest
	@echo ""
	@echo "═══════════════════════════════════════"
	@echo "  Step 2/3: Computing hadith families"
	@echo "═══════════════════════════════════════"
	cargo run -- analyze --families
	@echo ""
	@echo "═══════════════════════════════════════"
	@echo "  Step 3/3: Transmission analysis"
	@echo "═══════════════════════════════════════"
	cargo run -- analyze --juynboll
	@echo ""
	@echo "✓ Hadith pipeline complete."

# Ingest hadith only (no analysis)
hadith-ingest:
	cargo run -- ingest

# Full ingest with Ollama translation for remaining gaps
ingest-full:
	cargo run -- ingest --translate --translate-model command-r7b-arabic

# Ingest with sunnah.com English translations (no Ollama needed)
ingest:
	cargo run -- ingest

# Download Sanadset dataset (reference only — not used in current pipeline)
sanadset-download:
	@echo "Downloading Sanadset 650K (reference dataset, not used in current pipeline)..."
	@mkdir -p data
	curl -L -o /tmp/sanadset.zip "https://data.mendeley.com/public-api/zip/5xth87zwb5/download/5"
	cd /tmp && unzip -o sanadset.zip -d sanadset_tmp && mv sanadset_tmp/*.csv data/sanadset.csv
	@echo "✓ Sanadset downloaded to data/sanadset.csv"

# === Quran ingestion ===

# Use existing virtualenv if VIRTUAL_ENV is set, otherwise create .venv
ifdef VIRTUAL_ENV
VENV_PYTHON := $(VIRTUAL_ENV)/bin/python3
VENV_PIP := $(VIRTUAL_ENV)/bin/pip
else
VENV_PYTHON := .venv/bin/python3
VENV_PIP := .venv/bin/pip
endif

# Create virtual environment only if no venv is active and .venv doesn't exist
.venv/bin/python3:
	python3 -m venv .venv
	.venv/bin/pip install --upgrade pip
	.venv/bin/pip install pandas

# Ensure required packages are installed in the active venv
quran-prepare-deps:
	@echo "No Python dependencies needed (QUL data is local JSON)"

# ── QUL Data Sources (download manually from https://qul.tarteel.ai/resources) ──
#
# Text & Translations:
#   qul/qpc-hafs.json                        — QPC Hafs Arabic (ayah-by-ayah)       → resources/quran-script/86  (Simple JSON)
#   qul/en-sahih-international-simple.json    — Sahih International English           → resources/translation/193  (Simple JSON)
#   qul/qpc-hafs-word-by-word.json            — QPC Hafs Arabic (word-by-word)       → resources/quran-script/312 (JSON)
#   qul/en-tafisr-ibn-kathir.json              — Tafsir Ibn Kathir (English HTML)      → resources/tafsir/35        (JSON)
#   qul/colored-english-wbw-translation.json  — Colored English word-by-word          → resources/translation      (JSON)
#   qul/phrases.json                          — Mutashabihat ul Quran                → resources/mutashabihat     (JSON)
#   qul/matching-ayah.json                    — Similar ayahs                        → resources/similar-ayah     (JSON)
#
# Glyph Data (for Madani/Tajweed font modes):
#   qul/qpc-v2.json                           — QPC V2 glyph codes (word-by-word)   → resources/quran-script/61  (JSON)
#   qul/qpc-v4.json                           — QPC V4 tajweed glyphs (word-by-word) → resources/quran-script/47  (JSON)
#
# Fonts:
#   qul/UthmanicHafs_V22.woff2               — QPC Hafs Unicode font                → resources/font/245         (woff2)
#   ~/Downloads/"QPC V2 Font.woff2"/          — 604 per-page V2 fonts (Madani)       → resources/font/249         (woff2)
#   ~/Downloads/woff2/                        — 604 per-page V4 fonts (Tajweed)      → resources/font/240         (woff2)
#
# Font install (after downloading):
#   cp qul/UthmanicHafs_V22.woff2 frontend/static/fonts/UthmanicHafs.woff2
#   cp -r ~/Downloads/"QPC V2 Font.woff2"/ frontend/static/fonts/quran/v2/
#   cp -r ~/Downloads/woff2/ frontend/static/fonts/quran/v4/

# Prepare Quran data (QUL QPC Hafs + Tafsir Ibn Kathir, merge into CSV)
quran-prepare: $(if $(VIRTUAL_ENV),,$(VENV_PYTHON)) quran-prepare-deps
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

data/morphology-terms-ar.json:
	curl -sL https://raw.githubusercontent.com/mustafa0x/quran-morphology/master/morphology-terms-ar.json -o data/morphology-terms-ar.json

# Preflight check: verify all required data files exist before running pipeline
quran-check:
	@echo "Checking required data files..."
	@ok=true; \
	test -f qul/qpc-hafs.json                          && echo "  ✓ qul/qpc-hafs.json" || { echo "  ✗ qul/qpc-hafs.json — download from qul.tarteel.ai/resources/quran-script/86 (Simple JSON)"; ok=false; }; \
	test -f qul/en-sahih-international-simple.json     && echo "  ✓ qul/en-sahih-international-simple.json" || { echo "  ✗ qul/en-sahih-international-simple.json — download from qul.tarteel.ai/resources/translation/193 (Simple JSON)"; ok=false; }; \
	test -f qul/en-tafisr-ibn-kathir.json              && echo "  ✓ qul/en-tafisr-ibn-kathir.json" || { echo "  ✗ qul/en-tafisr-ibn-kathir.json — download from qul.tarteel.ai/resources/tafsir/35 (JSON)"; ok=false; }; \
	test -f data/quran.csv                              && echo "  ✓ data/quran.csv" || { echo "  ✗ data/quran.csv (run: make quran-prepare)"; ok=false; }; \
	test -f data/quran-morphology.txt                   && echo "  ✓ data/quran-morphology.txt (auto-downloaded)" || echo "  ○ data/quran-morphology.txt (will auto-download)"; \
	test -f qul/colored-english-wbw-translation.json   && echo "  ✓ qul/colored-english-wbw-translation.json" || { echo "  ✗ qul/colored-english-wbw-translation.json — download from qul.tarteel.ai/resources/translation (Colored English wbw translation → JSON)"; ok=false; }; \
	test -f qul/phrases.json                       && echo "  ✓ qul/phrases.json" || { echo "  ✗ qul/phrases.json — download from qul.tarteel.ai/resources/mutashabihat (JSON)"; ok=false; }; \
	test -f qul/matching-ayah.json                 && echo "  ✓ qul/matching-ayah.json" || { echo "  ✗ qul/matching-ayah.json — download from qul.tarteel.ai/resources/similar-ayah (JSON)"; ok=false; }; \
	test -f frontend/static/fonts/UthmanicHafs.woff2   && echo "  ✓ frontend/static/fonts/UthmanicHafs.woff2" || { echo "  ✗ UthmanicHafs font — cp qul/UthmanicHafs_V22.woff2 frontend/static/fonts/UthmanicHafs.woff2"; ok=false; }; \
	echo ""; \
	if $$ok; then echo "All required files present. Run: make quran-full"; else echo "⚠  Download missing files above before running make quran-full"; exit 1; fi

# 1. Ingest word morphology (must run before quran-similar so phrase text can be extracted)
#    Requires: data/quran-morphology.txt (auto-downloaded)
#    Optional: qul/colored-english-wbw-translation.json (download "Colored English wbw translation" JSON from qul.tarteel.ai/resources/translation)
quran-morphology:
	cargo run -- ingest-morphology

# 2. Ingest mutashabihat + similar ayahs from QUL JSON (must run after morphology)
#    Requires: qul/phrases.json + qul/matching-ayah.json (download from qul.tarteel.ai/resources)
quran-similar:
	cargo run -- ingest-quran-similar --qul-dir qul

# Base Quran pipeline: prepare data + ingest ayahs + hadith refs
quran: quran-prepare quran-ingest quran-hadith-refs

# Full Quran pipeline (ordered: check → prepare → ingest → hadith-refs → morphology → similar)
# Note: Manuscripts come from Corpus Coranicum live API (no ingestion needed)
quran-full: quran-check quran data/quran-morphology.txt data/morphology-terms-ar.json quran-morphology quran-similar


# === Analyze phase (runs on already-ingested data) ===

# Compute hadith families from embedding similarity
analyze-families:
	cargo run -- analyze --families

# Run all analysis: families
analyze:
	cargo run -- analyze --families

# Run transmission integrity analysis (CL/PCL + structural falsifiability tests)
analyze-transmission:
	cargo run -- analyze --juynboll

# Full pipeline: ingest 100 per book + all analysis
pipeline-test:
	cargo run -- ingest --limit 100
	cargo run -- analyze --families
	cargo run -- analyze --juynboll

# === Full pipeline (everything from scratch) ===

# Preflight check for entire pipeline
pipeline-check:
	@echo "Checking required data files..."
	@ok=true; \
	echo "── Hadith ──"; \
	test -f data/semantic_hadith.json                    && echo "  ✓ data/semantic_hadith.json" || { echo "  ✗ data/semantic_hadith.json (run: make semantic-setup)"; ok=false; }; \
	echo "── Quran ──"; \
	test -f data/quran.csv                              && echo "  ✓ data/quran.csv" || echo "  ○ data/quran.csv (will auto-generate via quran-prepare)"; \
	test -f data/quran-morphology.txt                   && echo "  ✓ data/quran-morphology.txt" || echo "  ○ data/quran-morphology.txt (will auto-download)"; \
	echo "── QUL (manual download from qul.tarteel.ai) ──"; \
	test -f qul/colored-english-wbw-translation.json    && echo "  ✓ qul/colored-english-wbw-translation.json" || { echo "  ✗ qul/colored-english-wbw-translation.json — download from qul.tarteel.ai/resources/translation (Colored English wbw translation → JSON)"; ok=false; }; \
	test -f qul/phrases.json                            && echo "  ✓ qul/phrases.json" || { echo "  ✗ qul/phrases.json — download from qul.tarteel.ai/resources/mutashabihat (JSON)"; ok=false; }; \
	test -f qul/matching-ayah.json                      && echo "  ✓ qul/matching-ayah.json" || { echo "  ✗ qul/matching-ayah.json — download from qul.tarteel.ai/resources/similar-ayah (JSON)"; ok=false; }; \
	echo "── Corpus Coranicum ──"; \
	test -d data/corpus-coranicum-tei                    && echo "  ✓ data/corpus-coranicum-tei/" || echo "  ○ data/corpus-coranicum-tei/ (will auto-clone from GitHub)"; \
	echo ""; \
	if $$ok; then echo "All required files present. Run: make pipeline-full"; else echo "⚠  Download missing files above first"; exit 1; fi

# Full pipeline: hadith + quran (everything from scratch)
pipeline-full: pipeline-check
	$(MAKE) hadith-full
	$(MAKE) quran-full
	@echo ""
	@echo "✓ Full pipeline complete. Run: make server"

# Clean all generated data
clean:
	rm -rf db_data target frontend/build frontend/node_modules .venv
