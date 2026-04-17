#!/usr/bin/env python3
"""
Fetch Awn al-Ma'bud - sharh Sunan Abu Dawud (book 5760) from turath.io API.

Usage:
  python3 scripts/fetch_awn_mabud.py            # headings only
  python3 scripts/fetch_awn_mabud.py --pages    # headings + all ~4665 pages

Outputs:
  data/awn_mabud_headings.json  - book metadata + headings
  data/awn_mabud_pages.json     - all pages (~4665 pages, ~55MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 5760
DISPLAY = "Awn al-Ma'bud"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "awn_mabud_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "awn_mabud_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
