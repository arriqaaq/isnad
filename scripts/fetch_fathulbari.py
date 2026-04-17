#!/usr/bin/env python3
"""
Fetch Fath al-Bari (book 1673) from turath.io API.

Usage:
  python3 scripts/fetch_fathulbari.py            # headings only
  python3 scripts/fetch_fathulbari.py --pages    # headings + all ~7996 pages

Outputs:
  data/fath_al_bari_headings.json  - book metadata + headings
  data/fath_al_bari_pages.json     - all pages (~7996 pages, ~90MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 1673
DISPLAY = "Fath al-Bari"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "fath_al_bari_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "fath_al_bari_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
