.PHONY: build frontend backend server dev stop download-data blog semantic-download semantic-extract semantic-verify semantic-setup ingest ingest-test ingest-full hadith-full hadith-ingest sanadset-download quran-prepare quran-prepare-deps quran-ingest quran-hadith-refs quran-morphology quran-similar quran quran-full quran-check turath-fetch-tafsir turath-fetch-fathulbari turath-fetch-nawawi turath-fetch-tuhfat turath-fetch-nasai turath-fetch-awnmabud turath-fetch-ibnmajah turath-fetch-tahdhib turath-fetch turath-mapping turath-mapping-narrators book-ingest-tafsir book-ingest-fathulbari book-ingest-nawawi book-ingest-tuhfat book-ingest-nasai book-ingest-awnmabud book-ingest-ibnmajah book-ingest-tahdhib book-ingest book-full pageindex-clone pageindex-deps pageindex-build pageindex-build-with-summaries pageindex-build-test pageindex-status analyze analyze-families analyze-transmission pipeline-check pipeline-test pipeline-full clean

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

# === Download pre-built data (skip ingestion) ===

# Download pre-built data and database from Google Drive (uses bge-m3 embeddings)
download-data:
	@echo "Downloading pre-built data from Google Drive..."
	uvx gdown "1X0oYLzCWytm0qTyjmZKAi_d-a0bvSv_d" -O /tmp/data.zip
	uvx gdown "16KOkdE5g7fGfH3zPwRmyGfzGUnHl444F" -O /tmp/db_data.zip
	@echo "Extracting data..."
	unzip -o /tmp/data.zip -d /tmp/ilm-extract
	unzip -o /tmp/db_data.zip -d /tmp/ilm-extract
	unzip -o /tmp/ilm-extract/ilm/data.zip -d .
	unzip -o /tmp/ilm-extract/ilm/db.zip -d .
	@rm -rf /tmp/data.zip /tmp/db_data.zip /tmp/ilm-extract
	@echo "✓ Data ready. Run: make dev"

# Build blog: convert articles/*.md to site HTML
blog:
	uv run --with markdown python3 scripts/build_blog.py

# === SemanticHadith KG data preparation (one-time) ===

# Download SemanticHadith KG V2 TTL (one-time, ~27MB compressed)
semantic-download:
	@mkdir -p /tmp
	curl -L -o /tmp/SemanticHadithKGV2.ttl.zip \
		https://github.com/A-Kamran/SemanticHadith-V2/raw/main/SemanticHadithKGV2.ttl.zip
	cd /tmp && unzip -o SemanticHadithKGV2.ttl.zip
	@echo "✓ SemanticHadith TTL extracted to /tmp/SemanticHadithKGV2.ttl"

# Extract TTL to JSON (one-time; rdflib is installed on-the-fly via uvx)
semantic-extract:
	uvx --with rdflib python3 scripts/build_semantic_data.py

# Verify extracted data (stdlib only)
semantic-verify:
	python3 scripts/verify_semantic_data.py

# Full SemanticHadith setup (download + extract + verify)
semantic-setup: semantic-download semantic-extract semantic-verify

# File-based target: auto-builds data/semantic_hadith.json by running the full
# semantic-setup chain when missing. Used as a dependency by pipeline-full so
# a fresh user doesn't have to know about semantic-setup at all.
data/semantic_hadith.json:
	$(MAKE) semantic-setup

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
	cargo run -- analyze --mustalah
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


# === Turath book viewer (Tafsir Ibn Kathir + Fath al-Bari) ===

# Fetch Tafsir Ibn Kathir from turath.io API (~7 min, resume-safe)
turath-fetch-tafsir:
	python3 scripts/fetch_tafsir.py --pages

# Fetch Fath al-Bari from turath.io API (~15 min, resume-safe)
turath-fetch-fathulbari:
	python3 scripts/fetch_fathulbari.py --pages

# Fetch Sharh Nawawi on Muslim from turath.io API (~8 min, resume-safe)
turath-fetch-nawawi:
	python3 scripts/fetch_nawawi.py --pages

# Fetch Tuhfat al-Ahwadhi (Tirmidhi sharh) from turath.io API (~10 min, resume-safe)
turath-fetch-tuhfat:
	python3 scripts/fetch_tuhfat_ahwadhi.py --pages

# Fetch Tahdhib al-Tahdhib (narrator bios) from turath.io API (~25 min, resume-safe)
turath-fetch-tahdhib:
	python3 scripts/fetch_tahdhib.py --pages

# Fetch Sahih Sunan al-Nasa'i from turath.io API (~2 min, resume-safe)
turath-fetch-nasai:
	python3 scripts/fetch_sahih_nasai.py --pages

# Fetch Awn al-Ma'bud (Abu Dawud sharh) from turath.io API (~8 min, resume-safe)
turath-fetch-awnmabud:
	python3 scripts/fetch_awn_mabud.py --pages

# Fetch Sunan Ibn Majah (Arnaut ed.) from turath.io API (~5 min, resume-safe)
turath-fetch-ibnmajah:
	python3 scripts/fetch_ibn_majah.py --pages

# Fetch all books (can run in parallel with -j8)
turath-fetch: turath-fetch-tafsir turath-fetch-fathulbari turath-fetch-nawawi turath-fetch-tuhfat turath-fetch-nasai turath-fetch-awnmabud turath-fetch-ibnmajah turath-fetch-tahdhib

# Build narrator→book mappings (needs: semantic_hadith.json + tahdhib_headings.json)
turath-mapping-narrators:
	python3 scripts/build_narrator_book_mapping.py

# Build hadith→sharh page mappings (needs: semantic_hadith.json + *_headings.json)
turath-mapping:
	python3 scripts/build_hadith_mapping.py
	python3 scripts/build_muslim_mapping.py
	python3 scripts/build_tirmidhi_mapping.py
	python3 scripts/build_nasai_mapping.py
	python3 scripts/build_abu_dawud_mapping.py
	python3 scripts/build_ibn_majah_mapping.py

# Ingest Tafsir Ibn Kathir (needs: turath-fetch-tafsir)
book-ingest-tafsir:
	cargo run -- ingest-book \
		--pages-file data/tafsir_ibn_kathir_pages.json \
		--headings-file data/tafsir_ibn_kathir_headings.json \
		--book-id 23604 \
		--name-ar "تفسير القرآن العظيم" \
		--name-en "Tafsir Ibn Kathir" \
		--author-ar "ابن كثير" \
		--tafsir-mapping data/tafsir_verse_mapping.json \
		--category quran --book-type tafsir

# Ingest Fath al-Bari (needs: turath-fetch-fathulbari + turath-mapping)
book-ingest-fathulbari:
	cargo run -- ingest-book \
		--pages-file data/fath_al_bari_pages.json \
		--headings-file data/fath_al_bari_headings.json \
		--book-id 1673 \
		--name-ar "فتح الباري بشرح البخاري" \
		--name-en "Fath al-Bari" \
		--author-ar "ابن حجر العسقلاني" \
		--sharh-mapping data/fath_al_bari_hadith_mapping.json \
		--sharh-collection-id 1 \
		--category hadith --book-type sharh

# Ingest Sharh Nawawi (needs: turath-fetch-nawawi + turath-mapping)
book-ingest-nawawi:
	cargo run -- ingest-book \
		--pages-file data/nawawi_on_muslim_pages.json \
		--headings-file data/nawawi_on_muslim_headings.json \
		--book-id 1711 \
		--name-ar "شرح النووي على مسلم" \
		--name-en "Sharh Nawawi on Muslim" \
		--author-ar "النووي" \
		--sharh-mapping data/nawawi_on_muslim_hadith_mapping.json \
		--sharh-collection-id 2 \
		--category hadith --book-type sharh

# Ingest Tuhfat al-Ahwadhi (needs: turath-fetch-tuhfat + turath-mapping)
book-ingest-tuhfat:
	cargo run -- ingest-book \
		--pages-file data/tuhfat_ahwadhi_pages.json \
		--headings-file data/tuhfat_ahwadhi_headings.json \
		--book-id 21662 \
		--name-ar "تحفة الأحوذي" \
		--name-en "Tuhfat al-Ahwadhi" \
		--author-ar "المباركفوري" \
		--sharh-mapping data/tuhfat_ahwadhi_hadith_mapping.json \
		--sharh-collection-id 4 \
		--category hadith --book-type sharh

# Ingest Sahih Sunan al-Nasa'i
book-ingest-nasai:
	cargo run -- ingest-book \
		--pages-file data/sahih_nasai_pages.json \
		--headings-file data/sahih_nasai_headings.json \
		--book-id 1147 \
		--name-ar "صحيح سنن النسائي" \
		--name-en "Sahih Sunan al-Nasai" \
		--author-ar "الألباني" \
		--sharh-mapping data/sahih_nasai_hadith_mapping.json \
		--sharh-collection-id 5 \
		--category hadith --book-type collection

# Ingest Awn al-Ma'bud
book-ingest-awnmabud:
	cargo run -- ingest-book \
		--pages-file data/awn_mabud_pages.json \
		--headings-file data/awn_mabud_headings.json \
		--book-id 5760 \
		--name-ar "عون المعبود شرح سنن أبي داود" \
		--name-en "Awn al-Mabud" \
		--author-ar "العظيم آبادي" \
		--sharh-mapping data/awn_mabud_hadith_mapping.json \
		--sharh-collection-id 3 \
		--category hadith --book-type sharh

# Ingest Sunan Ibn Majah
book-ingest-ibnmajah:
	cargo run -- ingest-book \
		--pages-file data/ibn_majah_pages.json \
		--headings-file data/ibn_majah_headings.json \
		--book-id 98138 \
		--name-ar "سنن ابن ماجه - ت الأرنؤوط" \
		--name-en "Sunan Ibn Majah" \
		--author-ar "ابن ماجه" \
		--sharh-mapping data/ibn_majah_hadith_mapping.json \
		--sharh-collection-id 6 \
		--category hadith --book-type collection

# Ingest Tahdhib al-Tahdhib (narrator bios)
book-ingest-tahdhib:
	cargo run -- ingest-book \
		--pages-file data/tahdhib_pages.json \
		--headings-file data/tahdhib_headings.json \
		--book-id 1278 \
		--name-ar "تهذيب التهذيب" \
		--name-en "Tahdhib al-Tahdhib" \
		--author-ar "ابن حجر العسقلاني" \
		--narrator-mapping data/tahdhib_narrator_mapping.json \
		--category narrator --book-type biography

# Ingest all books
book-ingest: book-ingest-tafsir book-ingest-fathulbari book-ingest-nawawi book-ingest-tuhfat book-ingest-nasai book-ingest-awnmabud book-ingest-ibnmajah book-ingest-tahdhib

# Full book pipeline: fetch → mapping → ingest
# Note: turath-mapping needs data/semantic_hadith.json (run make semantic-setup first if missing)
book-full: turath-fetch turath-mapping turath-mapping-narrators book-ingest

# Check required turath data files
turath-check:
	@echo "Checking turath data files..."
	@ok=true; \
	test -f data/semantic_hadith.json              && echo "  ✓ data/semantic_hadith.json" || { echo "  ✗ data/semantic_hadith.json (needed for hadith mapping — run: make semantic-setup)"; ok=false; }; \
	test -f data/tafsir_ibn_kathir_pages.json      && echo "  ✓ data/tafsir_ibn_kathir_pages.json" || echo "  ○ data/tafsir_ibn_kathir_pages.json (will fetch from turath.io)"; \
	test -f data/tafsir_verse_mapping.json         && echo "  ✓ data/tafsir_verse_mapping.json" || echo "  ○ data/tafsir_verse_mapping.json (built by fetch_tafsir.py)"; \
	test -f data/fath_al_bari_pages.json           && echo "  ✓ data/fath_al_bari_pages.json" || echo "  ○ data/fath_al_bari_pages.json (will fetch from turath.io)"; \
	test -f data/fath_al_bari_hadith_mapping.json  && echo "  ✓ data/fath_al_bari_hadith_mapping.json" || echo "  ○ data/fath_al_bari_hadith_mapping.json (built by build_hadith_mapping.py)"; \
	test -f data/nawawi_on_muslim_pages.json        && echo "  ✓ data/nawawi_on_muslim_pages.json" || echo "  ○ data/nawawi_on_muslim_pages.json (will fetch from turath.io)"; \
	test -f data/nawawi_on_muslim_hadith_mapping.json && echo "  ✓ data/nawawi_on_muslim_hadith_mapping.json" || echo "  ○ data/nawawi_on_muslim_hadith_mapping.json (built by build_muslim_mapping.py)"; \
	test -f data/tuhfat_ahwadhi_pages.json             && echo "  ✓ data/tuhfat_ahwadhi_pages.json" || echo "  ○ data/tuhfat_ahwadhi_pages.json (will fetch from turath.io)"; \
	test -f data/tuhfat_ahwadhi_hadith_mapping.json    && echo "  ✓ data/tuhfat_ahwadhi_hadith_mapping.json" || echo "  ○ data/tuhfat_ahwadhi_hadith_mapping.json (built by build_tirmidhi_mapping.py)"; \
	test -f data/sahih_nasai_pages.json                && echo "  ✓ data/sahih_nasai_pages.json" || echo "  ○ data/sahih_nasai_pages.json (will fetch from turath.io)"; \
	test -f data/sahih_nasai_hadith_mapping.json       && echo "  ✓ data/sahih_nasai_hadith_mapping.json" || echo "  ○ data/sahih_nasai_hadith_mapping.json (built by build_nasai_mapping.py)"; \
	test -f data/awn_mabud_pages.json                   && echo "  ✓ data/awn_mabud_pages.json" || echo "  ○ data/awn_mabud_pages.json (will fetch from turath.io)"; \
	test -f data/awn_mabud_hadith_mapping.json          && echo "  ✓ data/awn_mabud_hadith_mapping.json" || echo "  ○ data/awn_mabud_hadith_mapping.json (built by build_abu_dawud_mapping.py)"; \
	test -f data/ibn_majah_pages.json                  && echo "  ✓ data/ibn_majah_pages.json" || echo "  ○ data/ibn_majah_pages.json (will fetch from turath.io)"; \
	test -f data/ibn_majah_hadith_mapping.json         && echo "  ✓ data/ibn_majah_hadith_mapping.json" || echo "  ○ data/ibn_majah_hadith_mapping.json (built by build_ibn_majah_mapping.py)"; \
	test -f data/tahdhib_pages.json                    && echo "  ✓ data/tahdhib_pages.json" || echo "  ○ data/tahdhib_pages.json (will fetch from turath.io — ~25 min)"; \
	test -f data/tahdhib_narrator_mapping.json         && echo "  ✓ data/tahdhib_narrator_mapping.json" || echo "  ○ data/tahdhib_narrator_mapping.json (built by build_narrator_book_mapping.py)"; \
	echo ""; \
	if $$ok; then echo "Ready. Run: make turath-full"; else echo "⚠  Fix missing files above first"; exit 1; fi

# === Glossary extraction (one-time) ===

# Extract mustalah glossary from PDF (requires: pip install kreuzberg in .venv)
extract-glossary:
	.venv/bin/python3 scripts/extract_mustalah_glossary.py

# === Analyze phase (runs on already-ingested data) ===

# Compute hadith families from embedding similarity
analyze-families:
	cargo run -- analyze --families

# Run all analysis: families
analyze:
	cargo run -- analyze --families

# Run mustalah al-hadith transmission analysis
analyze-transmission:
	cargo run -- analyze --mustalah

# Full pipeline: ingest 100 per book + all analysis
pipeline-test:
	cargo run -- ingest --limit 100
	cargo run -- analyze --families
	cargo run -- analyze --mustalah

# === Full pipeline (everything from scratch) ===

# Preflight check for entire pipeline.
# Legend: ✓ present, ○ will be auto-fetched/built, ✗ MUST be downloaded manually.
pipeline-check:
	@echo "Checking required data files..."
	@ok=true; \
	echo "── Hadith ──"; \
	test -f data/semantic_hadith.json                    && echo "  ✓ data/semantic_hadith.json" || echo "  ○ data/semantic_hadith.json (will auto-build via semantic-setup; uses uvx for rdflib)"; \
	echo "── Quran ──"; \
	test -f data/quran.csv                              && echo "  ✓ data/quran.csv" || echo "  ○ data/quran.csv (will auto-generate via quran-prepare)"; \
	test -f data/quran-morphology.txt                   && echo "  ✓ data/quran-morphology.txt" || echo "  ○ data/quran-morphology.txt (will auto-download)"; \
	echo "── QUL (committed to repo; sourced from qul.tarteel.ai) ──"; \
	test -f qul/qpc-hafs.json                           && echo "  ✓ qul/qpc-hafs.json" || { echo "  ✗ qul/qpc-hafs.json missing — should ship with repo; restore via: git checkout HEAD -- qul/qpc-hafs.json"; ok=false; }; \
	test -f qul/en-sahih-international-simple.json      && echo "  ✓ qul/en-sahih-international-simple.json" || { echo "  ✗ qul/en-sahih-international-simple.json missing — restore via: git checkout HEAD -- qul/en-sahih-international-simple.json"; ok=false; }; \
	test -f qul/en-tafisr-ibn-kathir.json               && echo "  ✓ qul/en-tafisr-ibn-kathir.json" || { echo "  ✗ qul/en-tafisr-ibn-kathir.json missing — restore via: git checkout HEAD -- qul/en-tafisr-ibn-kathir.json"; ok=false; }; \
	test -f qul/colored-english-wbw-translation.json    && echo "  ✓ qul/colored-english-wbw-translation.json" || { echo "  ✗ qul/colored-english-wbw-translation.json missing — restore via: git checkout HEAD -- qul/colored-english-wbw-translation.json"; ok=false; }; \
	test -f qul/phrases.json                            && echo "  ✓ qul/phrases.json" || { echo "  ✗ qul/phrases.json missing — restore via: git checkout HEAD -- qul/phrases.json"; ok=false; }; \
	test -f qul/matching-ayah.json                      && echo "  ✓ qul/matching-ayah.json" || { echo "  ✗ qul/matching-ayah.json missing — restore via: git checkout HEAD -- qul/matching-ayah.json"; ok=false; }; \
	echo "── Quran fonts (committed to repo) ──"; \
	test -f frontend/static/fonts/UthmanicHafs.woff2    && echo "  ✓ frontend/static/fonts/UthmanicHafs.woff2" || { echo "  ✗ frontend/static/fonts/UthmanicHafs.woff2 missing — restore via: git checkout HEAD -- frontend/static/fonts/UthmanicHafs.woff2"; ok=false; }; \
	echo "── Corpus Coranicum ──"; \
	test -d data/corpus-coranicum-tei                    && echo "  ✓ data/corpus-coranicum-tei/" || echo "  ○ data/corpus-coranicum-tei/ (will auto-clone from GitHub)"; \
	echo "── PageIndex (sibling repo) ──"; \
	test -d ../PageIndex                                 && echo "  ✓ ../PageIndex" || echo "  ○ ../PageIndex (will auto-clone via pageindex-clone)"; \
	echo ""; \
	if $$ok; then echo "All required files present. Run: make pipeline-full"; else echo "⚠  Download missing files marked ✗ above before running make pipeline-full"; exit 1; fi

# === PageIndex book chat (markdown conversion + tree building) ===

# Auto-clone the VectifyAI/PageIndex repo into a sibling directory if missing.
# index_books.py adds ../PageIndex to sys.path and imports from it.
pageindex-clone:
	@if [ ! -d ../PageIndex ]; then \
		echo "Cloning PageIndex into ../PageIndex (one-time)..."; \
		git clone https://github.com/VectifyAI/PageIndex.git ../PageIndex; \
	else \
		echo "../PageIndex already present, skipping clone."; \
	fi

# Install PageIndex Python dependencies into .venv (one-time)
pageindex-deps: .venv/bin/python3 pageindex-clone
	$(VENV_PIP) install -r ../PageIndex/requirements.txt

# Build PageIndex trees from Turath JSON data (fast, no LLM needed)
pageindex-build: pageindex-deps
	$(VENV_PYTHON) scripts/index_books.py

# Build with Ollama-generated summaries (slower, better retrieval)
pageindex-build-with-summaries: pageindex-deps
	$(VENV_PYTHON) scripts/index_books.py --with-summaries

# Build a single book (for testing)
pageindex-build-test: pageindex-deps
	$(VENV_PYTHON) scripts/index_books.py --book-id 1673

# Show build status
pageindex-status: pageindex-deps
	$(VENV_PYTHON) scripts/index_books.py --status

# Full pipeline: hadith + quran + turath books (everything from scratch).
#
# Auto-fixed prerequisites (run as Makefile dependencies, no user action):
#   - data/semantic_hadith.json -> built via semantic-setup (which uses uvx for rdflib)
#   - ../PageIndex sibling repo  -> cloned via pageindex-clone
#
# Everything else (qul/*.json, UthmanicHafs.woff2) ships with the repo,
# so a fresh `git clone` followed by `make pipeline-full` should just work.
# pipeline-check is a defensive sanity check; it only fails if a tracked
# file was deleted locally.
pipeline-full: data/semantic_hadith.json pageindex-clone pipeline-check
	$(MAKE) hadith-full
	$(MAKE) quran-full
	$(MAKE) book-full
	$(MAKE) pageindex-build
	@echo ""
	@echo "✓ Full pipeline complete. Run: make server"

# Clean all generated data
clean:
	rm -rf db_data target frontend/build frontend/node_modules .venv
