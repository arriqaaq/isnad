#!/usr/bin/env python3
"""
Fetch Sharh Nawawi on Sahih Muslim (book 1711) from turath.io API.

Usage:
  python3 scripts/fetch_nawawi.py            # headings only
  python3 scripts/fetch_nawawi.py --pages    # headings + all ~4086 pages

Outputs:
  data/nawawi_on_muslim_headings.json  - book metadata + headings
  data/nawawi_on_muslim_pages.json     - all pages (~4086 pages, ~50MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 1711
DISPLAY = "Sharh Nawawi"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "nawawi_on_muslim_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "nawawi_on_muslim_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
