#!/usr/bin/env python3
"""
Fetch Tafsir Ibn Kathir (book 23604) from turath.io API.

Outputs:
  data/tafsir_ibn_kathir_headings.json  - book metadata + headings
  data/tafsir_ibn_kathir_pages.json     - all pages (full book)
  data/tafsir_verse_mapping.json        - surah:ayah -> page_index mapping
"""

import json
import os
import re
import sys

from _turath_fetch import (
    default_workers,
    fetch_all_pages,
    fetch_book_metadata,
)

BOOK_ID = 23604
DISPLAY = "Tafsir Ibn Kathir"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")

HEADINGS_FILE = os.path.join(DATA_DIR, "tafsir_ibn_kathir_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "tafsir_ibn_kathir_pages.json")
MAPPING_FILE = os.path.join(DATA_DIR, "tafsir_verse_mapping.json")


def build_verse_mapping(metadata: dict):
    """Parse headings to build surah:ayah -> page_index mapping."""
    indexes_obj = metadata.get("indexes", {})
    indexes = indexes_obj.get("headings", [])
    page_map = indexes_obj.get("page_map", [])
    print(f"Building verse mapping from {len(indexes)} headings...")

    # Pattern: [سورة NAME (SURAH_NUM): آية AYAH] or [سورة NAME (SURAH_NUM): الآيات AYAH إلى AYAH_END]
    # Arabic digits: ٠١٢٣٤٥٦٧٨٩
    # The API may use either Arabic or Western digits

    # Convert Arabic-Indic digits to Western
    def arabic_to_int(s: str) -> int:
        arabic_digits = "٠١٢٣٤٥٦٧٨٩"
        result = ""
        for ch in s.strip():
            idx = arabic_digits.find(ch)
            if idx >= 0:
                result += str(idx)
            elif ch.isdigit():
                result += ch
        return int(result) if result else 0

    # Multiple regex patterns to catch variations
    patterns = [
        # [سورة البقرة (٢): آية ١]
        re.compile(r'\[سورة\s+.+?\s*\(([٠-٩\d]+)\)\s*:\s*آية\s+([٠-٩\d]+)\]'),
        # [سورة البقرة (٢): الآيات ٨ إلى ٩]
        re.compile(r'\[سورة\s+.+?\s*\(([٠-٩\d]+)\)\s*:\s*الآيات\s+([٠-٩\d]+)\s*إلى\s*([٠-٩\d]+)\]'),
        # [سورة البقرة (٢): الآيات ٤] (single ayah with الآيات)
        re.compile(r'\[سورة\s+.+?\s*\(([٠-٩\d]+)\)\s*:\s*الآيات\s+([٠-٩\d]+)\]'),
    ]

    mapping = {}
    matched_count = 0
    unmatched_headings = []

    for heading in indexes:
        title = heading.get("title", "")
        page_index = heading.get("page", 0)  # This is the page_id in the indexes
        level = heading.get("level", 1)

        matched = False

        # Try range pattern first (most specific)
        m = patterns[1].search(title)
        if m:
            surah = arabic_to_int(m.group(1))
            ayah_start = arabic_to_int(m.group(2))
            ayah_end = arabic_to_int(m.group(3))
            for ayah in range(ayah_start, ayah_end + 1):
                key = f"{surah}:{ayah}"
                if key not in mapping:
                    mapping[key] = {
                        "page_index": page_index,
                        "heading": title,
                        "level": level
                    }
            matched_count += 1
            matched = True

        if not matched:
            # Try single آية pattern
            m = patterns[0].search(title)
            if m:
                surah = arabic_to_int(m.group(1))
                ayah = arabic_to_int(m.group(2))
                key = f"{surah}:{ayah}"
                if key not in mapping:
                    mapping[key] = {
                        "page_index": page_index,
                        "heading": title,
                        "level": level
                    }
                matched_count += 1
                matched = True

        if not matched:
            # Try single الآيات pattern (no range)
            m = patterns[2].search(title)
            if m:
                surah = arabic_to_int(m.group(1))
                ayah = arabic_to_int(m.group(2))
                key = f"{surah}:{ayah}"
                if key not in mapping:
                    mapping[key] = {
                        "page_index": page_index,
                        "heading": title,
                        "level": level
                    }
                matched_count += 1
                matched = True

        if not matched and "[سورة" in title:
            unmatched_headings.append(title)

    # Save mapping
    with open(MAPPING_FILE, "w", encoding="utf-8") as f:
        json.dump(mapping, f, ensure_ascii=False, indent=2)

    # Stats
    print(f"  Headings with verse references matched: {matched_count}")
    print(f"  Total ayah mappings: {len(mapping)}")
    print(f"  Saved to {MAPPING_FILE}")

    if unmatched_headings:
        print(f"\n  Unmatched headings with [سورة ({len(unmatched_headings)}):")
        for h in unmatched_headings[:20]:
            print(f"    {h}")
        if len(unmatched_headings) > 20:
            print(f"    ... and {len(unmatched_headings) - 20} more")

    # Coverage stats per surah
    print("\n  Coverage by surah (first 10):")
    surah_counts = {}
    for key in mapping:
        s = int(key.split(":")[0])
        surah_counts[s] = surah_counts.get(s, 0) + 1

    # Total ayahs per surah (approximate for coverage calc)
    total_ayahs = {
        1: 7, 2: 286, 3: 200, 4: 176, 5: 120, 6: 165, 7: 206, 8: 75, 9: 129, 10: 109,
        11: 123, 12: 111, 13: 43, 14: 52, 15: 99, 16: 128, 17: 111, 18: 110, 19: 98, 20: 135,
    }

    for s in sorted(surah_counts.keys())[:10]:
        total = total_ayahs.get(s, "?")
        mapped = surah_counts[s]
        print(f"    Surah {s}: {mapped} ayahs mapped (of {total})")

    print(f"\n  Total surahs with mappings: {len(surah_counts)}")

    return mapping


def main():
    os.makedirs(DATA_DIR, exist_ok=True)

    # Step 1: Fetch headings (delete cached file first if --force)
    if "--force" in sys.argv and os.path.exists(HEADINGS_FILE):
        os.remove(HEADINGS_FILE)
    metadata = fetch_book_metadata(BOOK_ID, DISPLAY, HEADINGS_FILE)

    # Step 2: Build verse mapping (from headings — fast, no API calls)
    print()
    build_verse_mapping(metadata)

    # Step 3: Fetch all pages (slow — concurrent fetch via shared helper)
    indexes_obj = metadata.get("indexes", {})
    page_map = indexes_obj.get("page_map", [])
    if "--pages" in sys.argv or "--all" in sys.argv:
        print()
        fetch_all_pages(
            metadata,
            BOOK_ID,
            PAGES_FILE,
            workers=default_workers(),
            display_name=DISPLAY,
        )
    else:
        print(f"\nSkipping page fetch (use --pages or --all to fetch all {len(page_map)} pages)")


if __name__ == "__main__":
    main()
