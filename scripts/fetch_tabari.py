#!/usr/bin/env python3
"""
Fetch Tafsir al-Tabari (Jami' al-Bayan, book 7798) from turath.io API.

Tabari uses a different heading convention than Ibn Kathir — no bracketed
[سورة X (N): آية M] refs. Instead:
  - The 118 lvl=1 headings split as: 4 front-matter + 114 surahs in Quran order.
    We map them positionally (skip the first 4), which bypasses Tabari's wild
    variety of surah title styles ("سورة X", "تفسير سورة \"...\"", "القول في
    تفسير السورة التي يذكر فيها X", etc.).
  - Ayah-level headings carry the ayah number at the end: ﴿…(N)﴾.
  - Phrase-level sub-headings (e.g. Al-Fatihah's ﴿بسم﴾) attach to the nearest
    preceding mapped ayah.
  - Missing ayahs inside a surah inherit the page of the preceding mapped ayah
    (forward-fill), giving 100% coverage over all 6,236 ayahs.

Usage:
  python3 scripts/fetch_tabari.py            # headings + verse mapping only
  python3 scripts/fetch_tabari.py --pages    # + all ~16,700 pages (large)

Outputs:
  data/tafsir_tabari_headings.json
  data/tafsir_tabari_pages.json
  data/tafsir_tabari_verse_mapping.json
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

BOOK_ID = 7798
DISPLAY = "Tafsir al-Tabari"
DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")

HEADINGS_FILE = os.path.join(DATA_DIR, "tafsir_tabari_headings.json")
PAGES_FILE = os.path.join(DATA_DIR, "tafsir_tabari_pages.json")
MAPPING_FILE = os.path.join(DATA_DIR, "tafsir_tabari_verse_mapping.json")

# Turath's edition prefaces the 114 surahs with 4 front-matter lvl=1 entries:
#   مقدمة التحقيق · مقدمة المصنف · القول في تأويل الاستعاذة · تفسير البسملة
EXPECTED_LVL1_TOTAL = 118
FRONT_MATTER_SKIP = 4

# Per-surah ayah counts from the canonical Hafs mus'haf (data/quran.csv).
AYAH_COUNTS = {
    1: 7, 2: 286, 3: 200, 4: 176, 5: 120, 6: 165, 7: 206, 8: 75, 9: 129, 10: 109,
    11: 123, 12: 111, 13: 43, 14: 52, 15: 99, 16: 128, 17: 111, 18: 110, 19: 98, 20: 135,
    21: 112, 22: 78, 23: 118, 24: 64, 25: 77, 26: 227, 27: 93, 28: 88, 29: 69, 30: 60,
    31: 34, 32: 30, 33: 73, 34: 54, 35: 45, 36: 83, 37: 182, 38: 88, 39: 75, 40: 85,
    41: 54, 42: 53, 43: 89, 44: 59, 45: 37, 46: 35, 47: 38, 48: 29, 49: 18, 50: 45,
    51: 60, 52: 49, 53: 62, 54: 55, 55: 78, 56: 96, 57: 29, 58: 22, 59: 24, 60: 13,
    61: 14, 62: 11, 63: 11, 64: 18, 65: 12, 66: 12, 67: 30, 68: 52, 69: 52, 70: 44,
    71: 28, 72: 28, 73: 20, 74: 56, 75: 40, 76: 31, 77: 50, 78: 40, 79: 46, 80: 42,
    81: 29, 82: 19, 83: 36, 84: 25, 85: 22, 86: 17, 87: 19, 88: 26, 89: 30, 90: 20,
    91: 15, 92: 21, 93: 11, 94: 8, 95: 8, 96: 19, 97: 5, 98: 8, 99: 8, 100: 11,
    101: 11, 102: 8, 103: 3, 104: 9, 105: 5, 106: 4, 107: 7, 108: 3, 109: 6, 110: 3,
    111: 5, 112: 4, 113: 5, 114: 6,
}

_AYAH_TAIL = re.compile(r'\((\d+|[٠-٩]+)\)\s*﴾')
_ARABIC_DIGITS = "٠١٢٣٤٥٦٧٨٩"


def _arab2int(s: str) -> int:
    s = s.strip()
    if s.isdigit():
        return int(s)
    out = []
    for ch in s:
        i = _ARABIC_DIGITS.find(ch)
        if i >= 0:
            out.append(str(i))
        elif ch.isdigit():
            out.append(ch)
    return int("".join(out)) if out else 0


def build_verse_mapping(metadata: dict) -> dict:
    """Positional surah dividers + trailing-paren ayah + forward-fill."""
    indexes_obj = metadata.get("indexes", {})
    headings = indexes_obj.get("headings", [])

    lvl1_idxs = [i for i, h in enumerate(headings) if h.get("level") == 1]
    if len(lvl1_idxs) != EXPECTED_LVL1_TOTAL:
        raise SystemExit(
            f"Tabari structure drift: expected {EXPECTED_LVL1_TOTAL} lvl=1 headings "
            f"(4 front-matter + 114 surahs), got {len(lvl1_idxs)}. "
            f"Re-inspect the turath index and update FRONT_MATTER_SKIP if needed."
        )

    surah_start_idxs = lvl1_idxs[FRONT_MATTER_SKIP:]
    assert len(surah_start_idxs) == 114

    surah_bounds = {}
    for i in range(114):
        start = surah_start_idxs[i]
        end = surah_start_idxs[i + 1] if i + 1 < 114 else len(headings)
        surah_bounds[i + 1] = (start, end)

    mapping: dict = {}
    stats = {"divider": 0, "direct": 0, "phrase": 0, "forward_fill": 0}

    for surah, (start, end) in surah_bounds.items():
        last_ayah = None
        for idx in range(start, end):
            h = headings[idx]
            title = h.get("title", "")
            page = h.get("page", 0)
            level = h.get("level", 1)

            if idx == start:
                mapping[f"{surah}:1"] = {
                    "page_index": page,
                    "heading": title,
                    "level": level,
                    "source": "divider",
                }
                stats["divider"] += 1
                continue

            m = _AYAH_TAIL.search(title)
            if m:
                n = _arab2int(m.group(1))
                if 1 <= n <= AYAH_COUNTS[surah]:
                    mapping[f"{surah}:{n}"] = {
                        "page_index": page,
                        "heading": title,
                        "level": level,
                        "source": "direct",
                    }
                    last_ayah = n
                    stats["direct"] += 1
                    continue

            if "﴿" in title and last_ayah is not None:
                key = f"{surah}:{last_ayah}"
                if mapping.get(key, {}).get("source") != "direct":
                    mapping[key] = {
                        "page_index": page,
                        "heading": title,
                        "level": level,
                        "source": "phrase",
                    }
                    stats["phrase"] += 1

    for s in range(1, 115):
        last_entry = None
        for a in range(1, AYAH_COUNTS[s] + 1):
            key = f"{s}:{a}"
            if key in mapping:
                last_entry = mapping[key]
            elif last_entry is not None:
                mapping[key] = {**last_entry, "source": "forward_fill"}
                stats["forward_fill"] += 1

    total = sum(AYAH_COUNTS.values())
    mapped = sum(1 for k in mapping)
    print(f"Building verse mapping from {len(headings)} headings...")
    print(f"  dividers:         {stats['divider']} / 114")
    print(f"  direct hits:      {stats['direct']}")
    print(f"  phrase backfills: {stats['phrase']}")
    print(f"  forward-filled:   {stats['forward_fill']}")
    print(f"  total mapped:     {mapped} / {total}  ({mapped / total * 100:.2f}%)")

    if mapped != total:
        missing = [
            f"{s}:{a}"
            for s in range(1, 115)
            for a in range(1, AYAH_COUNTS[s] + 1)
            if f"{s}:{a}" not in mapping
        ]
        raise SystemExit(f"Coverage < 100%; {len(missing)} ayahs missing, first few: {missing[:10]}")

    with open(MAPPING_FILE, "w", encoding="utf-8") as f:
        json.dump(mapping, f, ensure_ascii=False, indent=2)
    print(f"  Saved to {MAPPING_FILE}")
    return mapping


def main():
    os.makedirs(DATA_DIR, exist_ok=True)

    if "--force" in sys.argv and os.path.exists(HEADINGS_FILE):
        os.remove(HEADINGS_FILE)
    metadata = fetch_book_metadata(BOOK_ID, DISPLAY, HEADINGS_FILE)

    print()
    build_verse_mapping(metadata)

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
