#!/usr/bin/env python3
"""
Build Ibn Majah hadith_number → page_index mapping for Sunan Ibn Majah (Arnaut ed.).

Two methods for 100% coverage:
  1. Sequential باب alignment with Ibn Majah chapters (~4189 hadiths)
  2. Interpolation from nearest preceding mapped hadith (fills gaps)

Requires:
  data/ibn_majah_headings.json  (from fetch_ibn_majah.py)
  data/semantic_hadith.json     (source hadith data)

Outputs:
  data/ibn_majah_hadith_mapping.json
"""

import json
import os
import sys

DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "ibn_majah_headings.json")
HADITH_FILE = os.path.join(DATA_DIR, "semantic_hadith.json")
MAPPING_FILE = os.path.join(DATA_DIR, "ibn_majah_hadith_mapping.json")

# Ibn Majah is book 6 in BOOK_ORDER (SB=1, SM=2, SD=3, JT=4, SN=5, IM=6)
BOOK_PREFIX = "IM"


def main():
    for path, label in [
        (HEADINGS_FILE, "Ibn Majah headings"),
        (HADITH_FILE, "Semantic hadith data"),
    ]:
        if not os.path.exists(path):
            print(f"Error: {label} not found at {path}")
            sys.exit(1)

    print("Loading data...")
    with open(HEADINGS_FILE, "r", encoding="utf-8") as f:
        book_data = json.load(f)
    with open(HADITH_FILE, "r", encoding="utf-8") as f:
        hadith_data = json.load(f)

    headings = book_data["indexes"]["headings"]
    print(f"  Headings: {len(headings)}")

    # === Method 1: Sequential باب alignment ===
    print("Method 1: Sequential باب alignment...")

    bab_pages = []
    for h in headings:
        if "باب" in h["title"] and h["level"] == 2:
            bab_pages.append(h["page"] - 1)  # 0-based

    hadiths = {k: v for k, v in hadith_data["hadiths"].items() if v.get("book") == BOOK_PREFIX}
    chapters: dict[str, dict] = {}
    for k, h in hadiths.items():
        ch = h.get("chapter", "")
        ref = h.get("refNo", 0)
        if ch not in chapters:
            chapters[ch] = {"min": ref, "max": ref}
        chapters[ch]["min"] = min(chapters[ch]["min"], ref)
        chapters[ch]["max"] = max(chapters[ch]["max"], ref)

    sorted_chs = sorted(chapters.items(), key=lambda x: x[1]["min"])
    print(f"  باب headings: {len(bab_pages)}, our chapters: {len(sorted_chs)}")

    combined: dict[int, int] = {}
    sources: dict[int, str] = {}
    for i in range(min(len(sorted_chs), len(bab_pages))):
        ch_id, ch_data = sorted_chs[i]
        page_idx = bab_pages[i]
        for ref in range(ch_data["min"], ch_data["max"] + 1):
            combined[ref] = page_idx
            sources[ref] = "bab"

    print(f"  Mapped {len(combined)} hadiths via باب alignment")

    # === Method 2: Interpolation ===
    print("Method 2: Interpolating gaps...")
    all_refs = sorted(combined.keys())
    max_ref = max(h.get("refNo", 0) for h in hadiths.values())

    interp_count = 0
    for ref in range(1, max_ref + 1):
        if ref not in combined:
            prev = None
            for r in all_refs:
                if r <= ref:
                    prev = r
                else:
                    break
            if prev:
                combined[ref] = combined[prev]
                sources[ref] = "interpolated"
                interp_count += 1

    print(f"  Interpolated {interp_count} hadiths")

    hadith_refs = set(h.get("refNo", 0) for h in hadiths.values())
    covered = hadith_refs & set(combined.keys())

    output = {}
    for ref in sorted(combined.keys()):
        output[str(ref)] = {"page_index": combined[ref]}

    with open(MAPPING_FILE, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    bab_final = sum(1 for v in sources.values() if v == "bab")
    interp_final = sum(1 for v in sources.values() if v == "interpolated")

    print(f"\nResults:")
    print(f"  Method 1 (sequential باب):     {bab_final:>5} hadiths")
    print(f"  Method 2 (interpolated):       {interp_final:>5} hadiths")
    print(f"  Total mapped:                  {len(combined):>5}")
    print(f"  Coverage vs our data:          {len(covered)}/{len(hadith_refs)} = {len(covered)/len(hadith_refs)*100:.1f}%")
    print(f"  Saved to {MAPPING_FILE}")

    print(f"\nSpot checks:")
    for h_num in [1, 2, 50, 100, 500, 1000, 2000, 3000, 4300]:
        if h_num in combined:
            print(f"  Hadith {h_num:>5} -> page_index {combined[h_num]:>5} ({sources[h_num]})")
        else:
            print(f"  Hadith {h_num:>5} -> MISS")


if __name__ == "__main__":
    main()
