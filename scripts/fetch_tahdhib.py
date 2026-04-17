#!/usr/bin/env python3
"""
Fetch Tahdhib al-Tahdhib (book 1278) from turath.io API.

Usage:
  python3 scripts/fetch_tahdhib.py            # headings only
  python3 scripts/fetch_tahdhib.py --pages    # headings + all ~13120 pages

Outputs:
  data/tahdhib_headings.json  - book metadata + headings
  data/tahdhib_pages.json     - all pages (~13120 pages, ~150MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 1278
DISPLAY = "Tahdhib al-Tahdhib"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "tahdhib_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "tahdhib_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
