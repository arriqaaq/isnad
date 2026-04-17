#!/usr/bin/env python3
"""
Fetch Tuhfat al-Ahwadhi (book 21662) from turath.io API.

Usage:
  python3 scripts/fetch_tuhfat_ahwadhi.py            # headings only
  python3 scripts/fetch_tuhfat_ahwadhi.py --pages    # headings + all ~4874 pages

Outputs:
  data/tuhfat_ahwadhi_headings.json  - book metadata + headings
  data/tuhfat_ahwadhi_pages.json     - all pages (~4874 pages, ~60MB)
"""

import os

from _turath_fetch import run_book

BOOK_ID = 21662
DISPLAY = "Tuhfat al-Ahwadhi"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "tuhfat_ahwadhi_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "tuhfat_ahwadhi_pages.json")


def main():
    run_book(
        book_id=BOOK_ID,
        display_name=DISPLAY,
        headings_path=HEADINGS_FILE,
        pages_path=PAGES_FILE,
    )


if __name__ == "__main__":
    main()
