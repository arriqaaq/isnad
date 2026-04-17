#!/usr/bin/env python3
"""
Fetch Sahih Sunan al-Nasa'i (book 1147) from turath.io API.

Usage:
  python3 scripts/fetch_sahih_nasai.py            # headings only
  python3 scripts/fetch_sahih_nasai.py --pages    # headings + all ~1168 pages

Outputs:
  data/sahih_nasai_headings.json  - book metadata + headings
  data/sahih_nasai_pages.json     - all pages (~1168 pages, ~15MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 1147
DISPLAY = "Sahih Sunan al-Nasa'i"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "sahih_nasai_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "sahih_nasai_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
