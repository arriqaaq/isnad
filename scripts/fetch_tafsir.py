#!/usr/bin/env python3
"""
Fetch Tafsir Ibn Kathir (book 23604) from turath.io API.

Outputs:
  data/tafsir_ibn_kathir_headings.json  — book metadata + headings
  data/tafsir_ibn_kathir_pages.json     — all pages (full book)
  data/tafsir_verse_mapping.json        — surah:ayah → page_index mapping
"""

import json
import re
import time
import urllib.request
import sys
import os

BOOK_ID = 23604
API_BASE = "https://api.turath.io"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")

HEADINGS_FILE = os.path.join(DATA_DIR, "tafsir_ibn_kathir_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "tafsir_ibn_kathir_pages.json")
MAPPING_FILE = os.path.join(DATA_DIR, "tafsir_verse_mapping.json")


def fetch_json(url: str, retries: int = 3) -> dict:
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "hadith-project/1.0")
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except Exception as e:
            if attempt < retries - 1:
                wait = 2 ** (attempt + 1)
                print(f"  Retry {attempt+1} after error: {e} (waiting {wait}s)")
                time.sleep(wait)
            else:
                raise


def fetch_book_metadata():
    """Fetch book info + headings index."""
    print(f"Fetching book metadata for book_id={BOOK_ID}...")
    url = f"{API_BASE}/book?id={BOOK_ID}&include=indexes&ver=3"
    data = fetch_json(url)

    with open(HEADINGS_FILE, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    meta = data.get("meta", {})
    indexes = data.get("indexes", {})
    headings = indexes.get("headings", [])
    page_map = indexes.get("page_map", data.get("page_map", []))
    print(f"  Book: {meta.get('name', 'unknown')}")
    print(f"  Headings count: {len(headings)}")
    print(f"  Page map entries: {len(page_map)}")
    print(f"  Volumes: {indexes.get('volumes', [])}")
    print(f"  Saved to {HEADINGS_FILE}")

    return data


def fetch_all_pages(metadata: dict):
    """Fetch every page of the book."""
    # Determine total pages from page_map
    indexes_obj = metadata.get("indexes", {})
    page_map = indexes_obj.get("page_map", metadata.get("page_map", []))
    total = len(page_map)
    if total == 0:
        # Fallback: try fetching pages until we get empty
        print("No page_map found, will fetch until empty response")
        total = 5000  # generous upper bound

    print(f"Fetching all {total} pages...")

    pages = []
    empty_streak = 0

    # Resume support: if file exists, load and continue
    start_from = 1
    if os.path.exists(PAGES_FILE):
        try:
            with open(PAGES_FILE, "r", encoding="utf-8") as f:
                pages = json.load(f)
            if pages:
                start_from = pages[-1]["page_id"] + 1
                print(f"  Resuming from page_id {start_from} ({len(pages)} already fetched)")
        except (json.JSONDecodeError, KeyError):
            pages = []

    for page_id in range(start_from, total + 1):
        url = f"{API_BASE}/page?book_id={BOOK_ID}&pg={page_id}&ver=3"

        try:
            data = fetch_json(url)
        except Exception as e:
            print(f"\n  Error fetching page {page_id}: {e}")
            break

        if not data or (not data.get("text") and not data.get("meta")):
            empty_streak += 1
            if empty_streak > 10:
                print(f"\n  10 consecutive empty pages at page_id={page_id}, stopping.")
                break
            continue
        else:
            empty_streak = 0

        # meta may be a JSON string — parse it
        meta_raw = data.get("meta", {})
        if isinstance(meta_raw, str):
            try:
                meta_raw = json.loads(meta_raw)
            except json.JSONDecodeError:
                meta_raw = {}

        pages.append({
            "page_id": page_id,
            "meta": meta_raw,
            "text": data.get("text", "")
        })

        # Progress
        if len(pages) % 50 == 0:
            print(f"  Fetched {len(pages)} pages (page_id={page_id})...")
            # Save checkpoint
            with open(PAGES_FILE, "w", encoding="utf-8") as f:
                json.dump(pages, f, ensure_ascii=False)

        # Be polite: small delay between requests
        time.sleep(0.1)

    # Final save
    with open(PAGES_FILE, "w", encoding="utf-8") as f:
        json.dump(pages, f, ensure_ascii=False)

    print(f"  Total pages fetched: {len(pages)}")
    print(f"  Saved to {PAGES_FILE}")

    return pages


def build_verse_mapping(metadata: dict):
    """Parse headings to build surah:ayah → page_index mapping."""
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

    # Step 1: Fetch headings
    if os.path.exists(HEADINGS_FILE) and "--force" not in sys.argv:
        print(f"Headings already exist at {HEADINGS_FILE}, loading...")
        with open(HEADINGS_FILE, "r", encoding="utf-8") as f:
            metadata = json.load(f)
    else:
        metadata = fetch_book_metadata()

    # Step 2: Build verse mapping (from headings — fast, no API calls)
    print()
    build_verse_mapping(metadata)

    # Step 3: Fetch all pages (slow — ~2685 API calls)
    indexes_obj = metadata.get("indexes", {})
    page_map = indexes_obj.get("page_map", [])
    if "--pages" in sys.argv or "--all" in sys.argv:
        print()
        fetch_all_pages(metadata)
    else:
        print(f"\nSkipping page fetch (use --pages or --all to fetch all {len(page_map)} pages)")
        print("This will take ~7-15 minutes with rate limiting.")


if __name__ == "__main__":
    main()
