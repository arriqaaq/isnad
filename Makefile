.PHONY: build frontend backend server dev stop ingest ingest-test ingest-full list-books quran-prepare quran-ingest quran analyze analyze-bio analyze-families analyze-transmission pipeline-test pipeline-full clean

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

# Prepare Quran data (download Tanzil + Tafsir Ibn Kathir, merge into CSV)
quran-prepare:
	python3 scripts/prepare_quran_data.py

# Ingest Quran into SurrealDB (requires data/quran.csv from quran-prepare)
quran-ingest:
	cargo run -- ingest-quran

# Full Quran pipeline: prepare data + ingest
quran: quran-prepare quran-ingest

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
	cargo run -- analyze --narrator-bio data/ar_sanad_narrators.csv --families
	cargo run -- analyze --juynboll

# Clean all generated data
clean:
	rm -rf db_data target frontend/build frontend/node_modules
