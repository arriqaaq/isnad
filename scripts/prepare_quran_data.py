#!/usr/bin/env python3
"""
Load Quran text and Tafsir Ibn Kathir from QUL data files,
merge them into a single CSV for Rust ingestion.

Sources (all from https://qul.tarteel.ai/resources):
  - qul/qpc-hafs.json: Arabic (QPC Hafs), dict keyed by "surah:ayah"
  - qul/en-sahih-international-simple.json: English (Sahih International), dict keyed by "surah:ayah"
  - qul/en-tafisr-ibn-kathir.json: Tafsir Ibn Kathir (English HTML), dict keyed by "surah:ayah"

Output: data/quran.csv with columns: surah,ayah,text_ar,text_en,tafsir_en
"""

import csv
import json
import os
import re
import sys

OUTPUT_PATH = "data/quran.csv"


def load_qpc_arabic(path: str = "qul/qpc-hafs.json") -> dict[tuple[int, int], str]:
    """Load Arabic text from QUL QPC Hafs JSON, stripping trailing Arabic-Indic numerals.

    Returns {(surah, ayah): text_ar}.
    """
    with open(path, encoding="utf-8") as f:
        data = json.load(f)

    result = {}
    for key, entry in data.items():
        surah, ayah = int(entry["surah"]), int(entry["ayah"])
        text = entry["text"]
        # Strip trailing Arabic-Indic numerals (U+0660-U+066A) and whitespace
        text = re.sub(r"[\s\u0660-\u066a]+$", "", text)
        result[(surah, ayah)] = text
    return result


def load_qpc_english(path: str = "qul/en-sahih-international-simple.json") -> dict[tuple[int, int], str]:
    """Load English translation from QUL Sahih International JSON.

    Returns {(surah, ayah): text_en}.
    """
    with open(path, encoding="utf-8") as f:
        data = json.load(f)

    result = {}
    for key, entry in data.items():
        parts = key.split(":")
        surah, ayah = int(parts[0]), int(parts[1])
        result[(surah, ayah)] = entry["t"]
    return result


def load_qul_tafsir(path: str = "qul/en-tafisr-ibn-kathir.json") -> dict[tuple[int, int], str]:
    """Load Tafsir Ibn Kathir from QUL JSON.

    Values are either:
      - dict {"text": "HTML content"} — direct tafsir
      - str "surah:ayah" — reference to another verse's tafsir (grouped ayahs)

    Returns {(surah, ayah): tafsir_html}.
    """
    with open(path, encoding="utf-8") as f:
        data = json.load(f)

    # First pass: collect all direct tafsir entries
    direct = {}
    refs = {}
    for key, value in data.items():
        parts = key.split(":")
        surah, ayah = int(parts[0]), int(parts[1])
        if isinstance(value, dict):
            direct[(surah, ayah)] = value.get("text", "")
        elif isinstance(value, str):
            # Reference to another verse
            refs[(surah, ayah)] = value

    # Second pass: resolve references
    result = dict(direct)
    for (surah, ayah), ref_key in refs.items():
        ref_parts = ref_key.split(":")
        ref_surah, ref_ayah = int(ref_parts[0]), int(ref_parts[1])
        result[(surah, ayah)] = direct.get((ref_surah, ref_ayah), "")

    return result


def main():
    os.makedirs("data", exist_ok=True)

    print("Step 1: Load QUL Arabic and English data")
    ar_verses = load_qpc_arabic()
    en_verses = load_qpc_english()

    print(f"  Arabic verses: {len(ar_verses)}")
    print(f"  English verses: {len(en_verses)}")

    # Verify both have the same keys
    if ar_verses.keys() != en_verses.keys():
        ar_only = ar_verses.keys() - en_verses.keys()
        en_only = en_verses.keys() - ar_verses.keys()
        if ar_only:
            print(f"  WARNING: {len(ar_only)} verses in Arabic but not English: {list(ar_only)[:5]}")
        if en_only:
            print(f"  WARNING: {len(en_only)} verses in English but not Arabic: {list(en_only)[:5]}")

    print("Step 2: Load Tafsir Ibn Kathir from QUL")
    tafsir = load_qul_tafsir()
    print(f"  Tafsir entries: {len(tafsir)}")

    print("Step 3: Merge and write CSV")
    # Use Arabic keys as the canonical set (should be 6236)
    all_keys = sorted(ar_verses.keys())

    with open(OUTPUT_PATH, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["surah", "ayah", "text_ar", "text_en", "tafsir_en"])

        for surah, ayah in all_keys:
            text_ar = ar_verses.get((surah, ayah), "")
            text_en = en_verses.get((surah, ayah), "")
            tafsir_en = tafsir.get((surah, ayah), "")
            writer.writerow([surah, ayah, text_ar, text_en, tafsir_en])

    print(f"\nDone! Output: {OUTPUT_PATH}")
    print(f"  Total rows: {len(all_keys)}")

    # Assertions
    assert len(all_keys) == 6236, f"Expected 6236 verses, got {len(all_keys)}"

    # Check tafsir coverage
    tafsir_count = sum(1 for s, a in all_keys if tafsir.get((s, a), ""))
    print(f"  Verses with tafsir: {tafsir_count} / {len(all_keys)}")
    print(f"  Verses without tafsir: {len(all_keys) - tafsir_count}")


if __name__ == "__main__":
    main()
