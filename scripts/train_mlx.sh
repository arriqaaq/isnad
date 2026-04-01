#!/bin/bash
set -euo pipefail

# ---------------------------------------------------------------------------
# Train hadith-scholar model using MLX LoRA on Apple Silicon
#
# Prerequisites:
#   - macOS with Apple Silicon (M1/M2/M3/M4)
#   - data/train.jsonl + data/valid.jsonl (from prepare_training_data.py)
#   - ~16GB unified memory recommended (8GB works with smaller models)
#
# Usage:
#   bash scripts/train_mlx.sh              # Full pipeline: setup → train → fuse → convert
#   bash scripts/train_mlx.sh train        # Train only (venv must exist)
#   bash scripts/train_mlx.sh fuse         # Fuse only
#   bash scripts/train_mlx.sh convert      # Convert to GGUF only
# ---------------------------------------------------------------------------

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
VENV_DIR="$PROJECT_DIR/.venv-train"
MODEL="mlx-community/Phi-4-mini-instruct-4bit"
ADAPTER_PATH="$PROJECT_DIR/models/hadith-scholar-lora"
FUSED_PATH="$PROJECT_DIR/models/hadith-scholar-fused"
GGUF_PATH="$PROJECT_DIR/models/hadith-scholar.gguf"
GGUF_QUANTIZED="$PROJECT_DIR/models/hadith-scholar-q4km.gguf"
DATA_DIR="$PROJECT_DIR/data"
ITERS=1000
BATCH_SIZE=4
LORA_LAYERS=16

# ---------------------------------------------------------------------------
# Setup virtualenv
# ---------------------------------------------------------------------------
setup_venv() {
    if [ ! -d "$VENV_DIR" ]; then
        echo "Creating virtualenv at $VENV_DIR..."
        python3 -m venv "$VENV_DIR"
    fi
    # shellcheck disable=SC1091
    source "$VENV_DIR/bin/activate"
    echo "Installing dependencies..."
    pip install --quiet --upgrade pip
    pip install --quiet mlx-lm datasets
    echo "Virtualenv ready: $(python3 --version), mlx-lm installed"
}

# ---------------------------------------------------------------------------
# Train LoRA adapter
# ---------------------------------------------------------------------------
train() {
    # shellcheck disable=SC1091
    source "$VENV_DIR/bin/activate"

    if [ ! -f "$DATA_DIR/train.jsonl" ]; then
        echo "ERROR: $DATA_DIR/train.jsonl not found."
        echo "Run first: python3 scripts/prepare_training_data.py"
        exit 1
    fi

    echo ""
    echo "=== Training LoRA adapter ==="
    echo "  Model:       $MODEL"
    echo "  Data:        $DATA_DIR"
    echo "  Iterations:  $ITERS"
    echo "  Batch size:  $BATCH_SIZE"
    echo "  LoRA layers: $LORA_LAYERS"
    echo "  Output:      $ADAPTER_PATH"
    echo ""

    mlx_lm.lora \
        --model "$MODEL" \
        --train \
        --data "$DATA_DIR" \
        --iters "$ITERS" \
        --batch-size "$BATCH_SIZE" \
        --lora-layers "$LORA_LAYERS" \
        --adapter-path "$ADAPTER_PATH"

    echo ""
    echo "Training complete. Adapter saved to: $ADAPTER_PATH"
}

# ---------------------------------------------------------------------------
# Fuse LoRA weights into base model
# ---------------------------------------------------------------------------
fuse() {
    # shellcheck disable=SC1091
    source "$VENV_DIR/bin/activate"

    if [ ! -d "$ADAPTER_PATH" ]; then
        echo "ERROR: Adapter not found at $ADAPTER_PATH. Run training first."
        exit 1
    fi

    echo ""
    echo "=== Fusing LoRA adapter into base model ==="
    echo "  --de-quantize is required (4-bit can't be directly converted to GGUF)"
    echo ""

    mlx_lm.fuse \
        --model "$MODEL" \
        --adapter-path "$ADAPTER_PATH" \
        --save-path "$FUSED_PATH" \
        --de-quantize

    echo ""
    echo "Fused model saved to: $FUSED_PATH"
}

# ---------------------------------------------------------------------------
# Convert to GGUF for Ollama
# ---------------------------------------------------------------------------
convert() {
    # shellcheck disable=SC1091
    source "$VENV_DIR/bin/activate"

    if [ ! -d "$FUSED_PATH" ]; then
        echo "ERROR: Fused model not found at $FUSED_PATH. Run fuse first."
        exit 1
    fi

    LLAMA_CPP_DIR="$PROJECT_DIR/llama.cpp"
    if [ ! -d "$LLAMA_CPP_DIR" ]; then
        echo "Cloning llama.cpp..."
        git clone https://github.com/ggml-org/llama.cpp.git "$LLAMA_CPP_DIR"
    fi

    echo "Installing llama.cpp Python dependencies..."
    pip install --quiet -r "$LLAMA_CPP_DIR/requirements.txt" 2>/dev/null || true

    echo ""
    echo "=== Converting to GGUF ==="
    python3 "$LLAMA_CPP_DIR/convert_hf_to_gguf.py" "$FUSED_PATH" \
        --outfile "$GGUF_PATH" \
        --outtype q8_0

    echo ""
    echo "=== Quantizing to Q4_K_M ==="
    # Build llama-quantize if needed
    if [ ! -f "$LLAMA_CPP_DIR/build/bin/llama-quantize" ]; then
        echo "Building llama.cpp quantize tool..."
        cmake -B "$LLAMA_CPP_DIR/build" -S "$LLAMA_CPP_DIR" -DCMAKE_BUILD_TYPE=Release
        cmake --build "$LLAMA_CPP_DIR/build" --target llama-quantize -j
    fi

    "$LLAMA_CPP_DIR/build/bin/llama-quantize" "$GGUF_PATH" "$GGUF_QUANTIZED" Q4_K_M

    echo ""
    echo "GGUF models:"
    ls -lh "$GGUF_PATH" "$GGUF_QUANTIZED"
    echo ""
    echo "Next steps:"
    echo "  ollama create hadith-scholar -f models/Modelfile"
    echo "  ollama run hadith-scholar"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
STEP="${1:-all}"

case "$STEP" in
    setup)
        setup_venv
        ;;
    train)
        train
        ;;
    fuse)
        fuse
        ;;
    convert)
        convert
        ;;
    all)
        setup_venv
        train
        fuse
        convert
        ;;
    *)
        echo "Usage: $0 [setup|train|fuse|convert|all]"
        exit 1
        ;;
esac
