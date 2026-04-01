#!/usr/bin/env python3
"""
Download Quran text from Tanzil.net and Tafsir Ibn Kathir from HuggingFace,
merge them into a single CSV for Rust ingestion.

Sources:
  - Tanzil.net: Arabic (Uthmani) + English (Sahih International), pipe-delimited, 1-based
  - M-AI-C/en-tafsir-ibn-kathir: Tafsir only, sorah 1-based, ayah 0-based

Mapping: tafsir for Tanzil (S, V) = M-AI-C row where sorah=S AND ayah=V-1

Output: data/quran.csv with columns: surah,ayah,text_ar,text_en,tafsir_en
"""

import csv
import json
import os
import sys
import time
import urllib.request

TANZIL_AR_URL = "https://tanzil.net/pub/download/index.php?quranType=uthmani&outType=txt-2"
TANZIL_EN_URL = "https://tanzil.net/trans/en.sahih"
QURAN_COM_API = "https://api.quran.com/api/v4/quran/verses/uthmani_tajweed"
OUTPUT_PATH = "data/quran.csv"


def download(url: str, dest: str) -> None:
    if os.path.exists(dest):
        print(f"  Already exists: {dest}")
        return
    print(f"  Downloading {url} ...")
    urllib.request.urlretrieve(url, dest)
    print(f"  Saved to {dest}")


def parse_tanzil(path: str) -> dict[tuple[int, int], str]:
    """Parse Tanzil pipe-delimited file into {(surah, ayah): text}."""
    result = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split("|", 2)
            if len(parts) < 3:
                continue
            surah, ayah, text = int(parts[0]), int(parts[1]), parts[2]
            result[(surah, ayah)] = text
    return result


def load_tafsir() -> dict[tuple[int, int], str]:
    """Load tafsir from HuggingFace, return {(surah, tanzil_ayah): tafsir_text}.

    M-AI-C uses 0-based ayah, so we map: tanzil_ayah = m_aic_ayah + 1
    """
    try:
        from datasets import load_dataset
    except ImportError:
        print("ERROR: 'datasets' package not installed. Run: pip install datasets")
        sys.exit(1)

    print("  Loading M-AI-C/en-tafsir-ibn-kathir from HuggingFace...")
    ds = load_dataset("M-AI-C/en-tafsir-ibn-kathir", split="train")

    tafsir = {}
    for row in ds:
        sorah = int(row["sorah"])
        ayah_0based = int(row["ayah"])
        tanzil_ayah = ayah_0based + 1
        text = (row.get("en-tafsir-ibn-kathir-html") or "").strip()
        tafsir[(sorah, tanzil_ayah)] = text

    print(f"  Loaded {len(tafsir)} tafsir entries")
    return tafsir


def load_tajweed() -> dict[tuple[int, int], str]:
    """Download tajweed-annotated Arabic text from quran.com API v4.

    Returns {(surah, ayah): text_uthmani_tajweed}.
    """
    cache_path = "data/quran_tajweed_cache.json"

    # Use cache if available
    if os.path.exists(cache_path):
        print("  Loading tajweed from cache...")
        with open(cache_path, encoding="utf-8") as f:
            cached = json.load(f)
        result = {}
        for key, val in cached.items():
            s, a = key.split(":")
            result[(int(s), int(a))] = val
        print(f"  Loaded {len(result)} cached tajweed entries")
        return result

    print("  Downloading tajweed text from quran.com API (114 surahs)...")
    result = {}
    for chapter in range(1, 115):
        url = f"{QURAN_COM_API}?chapter_number={chapter}"
        try:
            req = urllib.request.Request(url, headers={
                "Accept": "application/json",
                "User-Agent": "HadithExplorer/1.0 (Quran data prep script)",
            })
            with urllib.request.urlopen(req) as resp:
                data = json.loads(resp.read().decode("utf-8"))
            for verse in data.get("verses", []):
                vk = verse.get("verse_key", "")
                text = verse.get("text_uthmani_tajweed", "")
                if ":" in vk and text:
                    s, a = vk.split(":")
                    result[(int(s), int(a))] = text
        except Exception as e:
            print(f"  WARNING: Failed to fetch chapter {chapter}: {e}")

        if chapter % 10 == 0:
            print(f"    {chapter}/114 surahs downloaded...")
        time.sleep(0.3)

    # Cache for future runs
    cache = {f"{s}:{a}": t for (s, a), t in result.items()}
    with open(cache_path, "w", encoding="utf-8") as f:
        json.dump(cache, f, ensure_ascii=False)

    print(f"  Downloaded {len(result)} tajweed entries")
    return result


def main():
    os.makedirs("data", exist_ok=True)

    ar_path = "data/quran_uthmani.txt"
    en_path = "data/quran_en_sahih.txt"

    print("Step 1: Download Tanzil data")
    download(TANZIL_AR_URL, ar_path)
    download(TANZIL_EN_URL, en_path)

    print("Step 2: Parse Tanzil files")
    ar_verses = parse_tanzil(ar_path)
    en_verses = parse_tanzil(en_path)

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

    print("Step 3: Load Tafsir Ibn Kathir")
    tafsir = load_tafsir()

    print("Step 4: Download tajweed text from quran.com")
    tajweed = load_tajweed()

    print("Step 5: Merge and write CSV")
    # Use Arabic keys as the canonical set (should be 6236)
    all_keys = sorted(ar_verses.keys())

    with open(OUTPUT_PATH, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["surah", "ayah", "text_ar", "text_en", "tafsir_en", "text_ar_tajweed"])

        for surah, ayah in all_keys:
            text_ar = ar_verses.get((surah, ayah), "")
            text_en = en_verses.get((surah, ayah), "")
            tafsir_en = tafsir.get((surah, ayah), "")
            text_ar_tajweed = tajweed.get((surah, ayah), "")
            writer.writerow([surah, ayah, text_ar, text_en, tafsir_en, text_ar_tajweed])

    print(f"\nDone! Output: {OUTPUT_PATH}")
    print(f"  Total rows: {len(all_keys)}")

    # Assertions
    assert len(all_keys) == 6236, f"Expected 6236 verses, got {len(all_keys)}"
    assert len(all_keys) == len(set(all_keys)), "Duplicate keys found!"

    # Check tafsir coverage
    tafsir_count = sum(1 for s, a in all_keys if tafsir.get((s, a), ""))
    print(f"  Verses with tafsir: {tafsir_count} / {len(all_keys)}")
    print(f"  Verses without tafsir: {len(all_keys) - tafsir_count}")

    # Check tajweed coverage
    tajweed_count = sum(1 for s, a in all_keys if tajweed.get((s, a), ""))
    print(f"  Verses with tajweed: {tajweed_count} / {len(all_keys)}")

    # Verify edge cases
    assert (1, 1) in ar_verses, "Missing Al-Fatihah verse 1 (Bismillah)"
    assert tafsir.get((1, 1), "") == "", "Expected no tafsir for 1:1 (Bismillah)"
    print("  Edge case: 1:1 (Bismillah) has no tafsir ✓")

    assert (2, 1) in ar_verses, "Missing Al-Baqarah verse 1"
    assert tafsir.get((2, 1), "") != "", "Expected tafsir for 2:1"
    print("  Edge case: 2:1 has tafsir ✓")


if __name__ == "__main__":
    main()
