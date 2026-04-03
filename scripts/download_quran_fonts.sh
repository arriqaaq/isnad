#!/usr/bin/env bash
# Download QCF (Quran Complex Font) per-page font files from quran.com
# V2 = black Madani glyphs, V4 = colored tajweed glyphs (COLRv1)
# 604 pages × 2 variants = 1,208 font files (~85MB total)

set -euo pipefail

BASE="https://quran.com/fonts/quran/hafs"
V2_DIR="frontend/static/fonts/quran/v2"
V4_DIR="frontend/static/fonts/quran/v4"
PAGES=604

mkdir -p "$V2_DIR" "$V4_DIR"

echo "Downloading QCF V2 fonts (Madani, black glyphs)..."
for i in $(seq 1 $PAGES); do
  f="p${i}.woff2"
  if [ ! -f "$V2_DIR/$f" ]; then
    curl -sf "$BASE/v2/woff2/$f" -o "$V2_DIR/$f" && echo "  v2/$f" || echo "  FAILED: v2/$f"
  fi
done

echo "Downloading QCF V4 fonts (Tajweed, colored glyphs)..."
for i in $(seq 1 $PAGES); do
  f="p${i}.woff2"
  if [ ! -f "$V4_DIR/$f" ]; then
    curl -sf "$BASE/v4/colrv1/woff2/$f" -o "$V4_DIR/$f" && echo "  v4/$f" || echo "  FAILED: v4/$f"
  fi
done

v2_count=$(ls "$V2_DIR"/*.woff2 2>/dev/null | wc -l | tr -d ' ')
v4_count=$(ls "$V4_DIR"/*.woff2 2>/dev/null | wc -l | tr -d ' ')
echo ""
echo "Done: $v2_count V2 fonts, $v4_count V4 fonts"
