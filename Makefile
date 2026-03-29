.PHONY: build frontend backend server dev stop ingest ingest-test ingest-full list-books clean

# Build everything
build: backend frontend

# Build frontend
frontend:
	cd frontend && npm install && npm run build

# Build backend
backend:
	cargo build

# Start server (foreground)
server: build
	cargo run -- serve --port 3000

# Build everything and start server in background
dev: build
	@echo "Starting server on http://localhost:3000..."
	@cargo run -- serve --port 3000 &
	@sleep 2
	@echo "Server running at http://localhost:3000 (use 'make stop' to shut down)"

# Stop background server
stop:
	@pkill -f "target/debug/hadith serve" 2>/dev/null && echo "Server stopped" || echo "No server running"

# List available books in dataset
list-books:
	cargo run -- ingest --list-books

# Quick test ingest (5 per book, with translation)
ingest-test:
	rm -rf db_data
	cargo run -- ingest --limit 5 --translate

# Full ingest of 6 major books with translation
ingest-full:
	rm -rf db_data
	cargo run -- ingest --translate

# Ingest with human English translations (no Ollama needed)
ingest:
	rm -rf db_data
	cargo run -- ingest

# Clean all generated data
clean:
	rm -rf db_data target frontend/build frontend/node_modules
