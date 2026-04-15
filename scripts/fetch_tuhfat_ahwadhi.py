#!/usr/bin/env python3
"""
Fetch Tuhfat al-Ahwadhi (book 21662) from turath.io API.

Usage:
  python3 scripts/fetch_tuhfat_ahwadhi.py            # headings only
  python3 scripts/fetch_tuhfat_ahwadhi.py --pages     # headings + all 4874 pages (~10 min)

Outputs:
  data/tuhfat_ahwadhi_headings.json  — book metadata + headings (1853 headings)
  data/tuhfat_ahwadhi_pages.json     — all pages (~4874 pages, ~60MB)
"""

import json
import time
import urllib.request
import sys
import os

BOOK_ID = 21662
API_BASE = "https://api.turath.io"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")

HEADINGS_FILE = os.path.join(DATA_DIR, "tuhfat_ahwadhi_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "tuhfat_ahwadhi_pages.json")


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
    if os.path.exists(HEADINGS_FILE):
        print(f"Headings already exist at {HEADINGS_FILE}, loading...")
        with open(HEADINGS_FILE, "r", encoding="utf-8") as f:
            return json.load(f)

    print(f"Fetching book metadata for Tuhfat al-Ahwadhi (book_id={BOOK_ID})...")
    url = f"{API_BASE}/book?id={BOOK_ID}&include=indexes&ver=3"
    data = fetch_json(url)

    with open(HEADINGS_FILE, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    meta = data.get("meta", {})
    indexes = data.get("indexes", {})
    headings = indexes.get("headings", [])
    page_map = indexes.get("page_map", [])
    print(f"  Book: {meta.get('name', 'unknown')}")
    print(f"  Headings count: {len(headings)}")
    print(f"  Page map entries: {len(page_map)}")
    print(f"  Volumes: {indexes.get('volumes', [])}")
    print(f"  Saved to {HEADINGS_FILE}")
    return data


def fetch_all_pages(metadata: dict):
    indexes_obj = metadata.get("indexes", {})
    page_map = indexes_obj.get("page_map", [])
    total = len(page_map)
    if total == 0:
        total = 5000

    print(f"Fetching all {total} pages for Tuhfat al-Ahwadhi...")

    pages = []
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

    empty_streak = 0
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

        if len(pages) % 100 == 0:
            print(f"  Fetched {len(pages)} pages (page_id={page_id})...")
            with open(PAGES_FILE, "w", encoding="utf-8") as f:
                json.dump(pages, f, ensure_ascii=False)

        time.sleep(0.1)

    with open(PAGES_FILE, "w", encoding="utf-8") as f:
        json.dump(pages, f, ensure_ascii=False)

    print(f"  Total pages fetched: {len(pages)}")
    print(f"  Saved to {PAGES_FILE}")
    return pages


def main():
    os.makedirs(DATA_DIR, exist_ok=True)
    metadata = fetch_book_metadata()

    if "--pages" in sys.argv:
        fetch_all_pages(metadata)
    else:
        indexes = metadata.get("indexes", {})
        page_map = indexes.get("page_map", [])
        print(f"\nSkipping page fetch. Use --pages to fetch all {len(page_map)} pages (~10 min).")


if __name__ == "__main__":
    main()
