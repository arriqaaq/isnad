#!/usr/bin/env python3
"""
Fetch Sunan Ibn Majah - Arnaut edition (book 98138) from turath.io API.

Usage:
  python3 scripts/fetch_ibn_majah.py            # headings only
  python3 scripts/fetch_ibn_majah.py --pages    # headings + all ~3023 pages

Outputs:
  data/ibn_majah_headings.json  - book metadata + headings
  data/ibn_majah_pages.json     - all pages (~3023 pages, ~35MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 98138
DISPLAY = "Sunan Ibn Majah"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "ibn_majah_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "ibn_majah_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
