#!/usr/bin/env python3
"""
Build hadith_number → page_index mapping for Fath al-Bari.

Three methods combined for 100% coverage:
  1. Direct [الحديث N] markers in page text (exact, ~1620 hadiths)
  2. Sequential باب alignment with Bukhari chapters (~5165 hadiths)
  3. Interpolation from nearest preceding mapped hadith (~779 hadiths)

Requires:
  data/fath_al_bari_pages.json     (from fetch_fathulbari.py --pages)
  data/fath_al_bari_headings.json  (from fetch_fathulbari.py)
  data/semantic_hadith.json        (source hadith data)

Outputs:
  data/fath_al_bari_hadith_mapping.json
"""

import json
import re
import os
import sys
import unicodedata

DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
PAGES_FILE = os.path.join(DATA_DIR, "fath_al_bari_pages.json")
HEADINGS_FILE = os.path.join(DATA_DIR, "fath_al_bari_headings.json")
HADITH_FILE = os.path.join(DATA_DIR, "semantic_hadith.json")
MAPPING_FILE = os.path.join(DATA_DIR, "fath_al_bari_hadith_mapping.json")

BUKHARI_TOTAL = 7563


def strip_diacritics(text: str) -> str:
    return "".join(c for c in text if unicodedata.category(c) != "Mn")


def method1_direct_markers(pages: list) -> dict[int, int]:
    """Scan page text for [الحديث N] markers."""
    print("Method 1: Scanning for [الحديث N] markers...")
    pattern = re.compile(r"\[الحديث\s*(\d+)")
    mapping = {}
    for page in pages:
        page_index = page["page_id"] - 1
        stripped = strip_diacritics(page.get("text", ""))
        for m in pattern.findall(stripped):
            num = int(m)
            if num not in mapping:
                mapping[num] = page_index
    print(f"  Found {len(mapping)} unique hadith numbers")
    return mapping


def method2_bab_alignment(headings_data: dict, hadith_data: dict) -> dict[int, int]:
    """Match sequential باب headings to Bukhari chapter order."""
    print("Method 2: Sequential باب alignment...")
    headings = headings_data["indexes"]["headings"]

    # Extract ordered باب entries from the commentary section (page >= 490)
    bab_pages = []
    for h in headings:
        if h["page"] < 490:
            continue
        if re.match(r"\d+\s*-\s*باب", h["title"]) and h["level"] == 2:
            bab_pages.append(h["page"] - 1)  # 0-based

    # Group Bukhari hadiths by chapter, ordered by min hadith number
    bukhari = {k: v for k, v in hadith_data["hadiths"].items() if v.get("book") == "SB"}
    chapters: dict[str, dict] = {}
    for k, h in bukhari.items():
        ch = h.get("chapter", "")
        ref = h.get("refNo", 0)
        if ch not in chapters:
            chapters[ch] = {"min_ref": ref, "max_ref": ref}
        chapters[ch]["min_ref"] = min(chapters[ch]["min_ref"], ref)
        chapters[ch]["max_ref"] = max(chapters[ch]["max_ref"], ref)

    sorted_chapters = sorted(chapters.items(), key=lambda x: x[1]["min_ref"])

    # Map each chapter's hadiths to the corresponding باب page
    mapping = {}
    for i in range(min(len(sorted_chapters), len(bab_pages))):
        ch_id, ch_data = sorted_chapters[i]
        page_idx = bab_pages[i]
        for ref in range(ch_data["min_ref"], ch_data["max_ref"] + 1):
            mapping[ref] = page_idx

    print(f"  Mapped {len(mapping)} hadiths via {len(bab_pages)} باب headings")
    return mapping


def method3_interpolation(combined: dict[int, int]) -> dict[int, int]:
    """Fill remaining gaps by using nearest preceding mapped hadith's page."""
    print("Method 3: Interpolating gaps...")
    all_refs = sorted(combined.keys())
    additions = {}
    for ref in range(1, BUKHARI_TOTAL + 1):
        if ref not in combined:
            prev_ref = None
            for r in all_refs:
                if r <= ref:
                    prev_ref = r
                else:
                    break
            if prev_ref:
                additions[ref] = combined[prev_ref]
    print(f"  Interpolated {len(additions)} hadiths")
    return additions


def main():
    for path, label in [
        (PAGES_FILE, "Fath al-Bari pages"),
        (HEADINGS_FILE, "Fath al-Bari headings"),
        (HADITH_FILE, "Semantic hadith data"),
    ]:
        if not os.path.exists(path):
            print(f"Error: {label} not found at {path}")
            sys.exit(1)

    print(f"Loading data...")
    with open(PAGES_FILE, "r", encoding="utf-8") as f:
        pages = json.load(f)
    with open(HEADINGS_FILE, "r", encoding="utf-8") as f:
        headings_data = json.load(f)
    with open(HADITH_FILE, "r", encoding="utf-8") as f:
        hadith_data = json.load(f)
    print(f"  Pages: {len(pages)}, Headings: {len(headings_data['indexes']['headings'])}")

    # Run all three methods
    m1 = method1_direct_markers(pages)
    m2 = method2_bab_alignment(headings_data, hadith_data)
    m3_base: dict[int, int] = {}

    # Combine: prefer method 1 (exact), then method 2 (باب), then method 3 (interpolation)
    combined: dict[int, int] = {}
    sources: dict[int, str] = {}
    for ref in range(1, BUKHARI_TOTAL + 1):
        if ref in m1:
            combined[ref] = m1[ref]
            sources[ref] = "direct"
        elif ref in m2:
            combined[ref] = m2[ref]
            sources[ref] = "bab"

    interpolated = method3_interpolation(combined)
    for ref, page_idx in interpolated.items():
        combined[ref] = page_idx
        sources[ref] = "interpolated"

    # Save
    output = {}
    for ref in sorted(combined.keys()):
        output[str(ref)] = {"page_index": combined[ref]}

    with open(MAPPING_FILE, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    # Stats
    direct_count = sum(1 for v in sources.values() if v == "direct")
    bab_count = sum(1 for v in sources.values() if v == "bab")
    interp_count = sum(1 for v in sources.values() if v == "interpolated")

    print(f"\nResults:")
    print(f"  Method 1 (direct [الحديث N]):  {direct_count:>5} hadiths")
    print(f"  Method 2 (sequential باب):     {bab_count:>5} hadiths")
    print(f"  Method 3 (interpolated):       {interp_count:>5} hadiths")
    print(f"  Total mapped:                  {len(combined):>5} / {BUKHARI_TOTAL}")
    print(f"  Coverage:                      {len(combined)/BUKHARI_TOTAL*100:.1f}%")
    print(f"  Saved to {MAPPING_FILE}")

    # Spot checks
    print(f"\nSpot checks:")
    for h_num in [1, 2, 9, 100, 1000, 3000, 5000, 7000, 7500]:
        if h_num in combined:
            print(f"  Hadith {h_num:>5} -> page_index {combined[h_num]:>5} ({sources[h_num]})")
        else:
            print(f"  Hadith {h_num:>5} -> MISS")


if __name__ == "__main__":
    main()
