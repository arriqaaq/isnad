#!/usr/bin/env python3
"""
Build narrator → Tahdhib al-Tahdhib page mapping.

Matches our narrators (from semantic_hadith.json) to Tahdhib heading entries
using normalized Arabic name matching (exact, prefix-3, prefix-2).

Requires:
  data/tahdhib_headings.json   (from fetch_tahdhib.py)
  data/semantic_hadith.json    (source narrator data)

Outputs:
  data/tahdhib_narrator_mapping.json
"""

import json
import re
import os
import sys
import unicodedata

DATA_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "data")
HEADINGS_FILE = os.path.join(DATA_DIR, "tahdhib_headings.json")
HADITH_FILE = os.path.join(DATA_DIR, "semantic_hadith.json")
MAPPING_FILE = os.path.join(DATA_DIR, "tahdhib_narrator_mapping.json")

TURATH_BOOK_ID = 1278
BOOK_NAME = "Tahdhib al-Tahdhib"


def strip_diacritics(s: str) -> str:
    return "".join(c for c in s if unicodedata.category(c) != "Mn")


def normalize(s: str) -> str:
    s = strip_diacritics(s)
    s = s.replace("أ", "ا").replace("إ", "ا").replace("آ", "ا")
    s = s.replace("ة", "ه").replace("ى", "ي")
    s = re.sub(r"\s+", " ", s).strip()
    return s


def name_parts(s: str, n: int) -> str:
    parts = s.split()
    return " ".join(parts[:min(n, len(parts))])


def main():
    for path, label in [
        (HEADINGS_FILE, "Tahdhib headings"),
        (HADITH_FILE, "Semantic hadith data"),
    ]:
        if not os.path.exists(path):
            print(f"Error: {label} not found at {path}")
            sys.exit(1)

    print("Loading data...")
    with open(HEADINGS_FILE, "r", encoding="utf-8") as f:
        tahdhib_data = json.load(f)
    with open(HADITH_FILE, "r", encoding="utf-8") as f:
        hadith_data = json.load(f)

    headings = tahdhib_data["indexes"]["headings"]
    narrators = hadith_data.get("narrators", {})
    print(f"  Tahdhib headings: {len(headings)}")
    print(f"  Our narrators: {len(narrators)}")

    # Extract Tahdhib narrator entries from headings: [N] name pattern at level 2
    print("\nExtracting Tahdhib narrator entries from headings...")
    tahdhib_entries = []
    for h in headings:
        m = re.match(r"\[(\d+)\]\s*(.+)", h["title"])
        if m and h["level"] == 2:
            raw_name = m.group(2).strip()
            # Remove book symbols like (خ د تم س ق)
            clean = re.sub(r"^\([^)]*\)\s*", "", raw_name).strip()
            tahdhib_entries.append({
                "num": int(m.group(1)),
                "name": clean,
                "name_norm": normalize(clean),
                "page": h["page"] - 1,  # 0-based
            })

    print(f"  Tahdhib narrator entries: {len(tahdhib_entries)}")

    # Build search indices
    idx_full: dict[str, dict] = {}
    idx_3: dict[str, dict] = {}
    idx_2: dict[str, dict] = {}
    for e in tahdhib_entries:
        k = e["name_norm"]
        if k not in idx_full:
            idx_full[k] = e
        k3 = name_parts(k, 3)
        if k3 not in idx_3:
            idx_3[k3] = e
        k2 = name_parts(k, 2)
        if k2 not in idx_2:
            idx_2[k2] = e

    # Build our narrator list
    our = []
    for nid, n in narrators.items():
        name = n.get("name", "")
        popular = n.get("popularName", "")
        if name:
            our.append({
                "id": nid,
                "name": name,
                "name_norm": normalize(name),
                "popular_norm": normalize(popular) if popular else "",
            })

    print(f"  Our narrators with names: {len(our)}")

    # Match
    print("\nMatching...")
    mapping = {}
    exact_count = 0
    prefix3_count = 0
    prefix2_count = 0

    for n in our:
        matched = None

        # Exact full name
        if n["name_norm"] in idx_full:
            matched = idx_full[n["name_norm"]]
            exact_count += 1
        elif n["popular_norm"] and n["popular_norm"] in idx_full:
            matched = idx_full[n["popular_norm"]]
            exact_count += 1

        # Prefix 3 words
        if not matched:
            k3 = name_parts(n["name_norm"], 3)
            if k3 in idx_3 and len(k3) > 8:
                matched = idx_3[k3]
                prefix3_count += 1
            elif n["popular_norm"]:
                pk3 = name_parts(n["popular_norm"], 3)
                if pk3 in idx_3 and len(pk3) > 8:
                    matched = idx_3[pk3]
                    prefix3_count += 1

        # Prefix 2 words
        if not matched:
            k2 = name_parts(n["name_norm"], 2)
            if k2 in idx_2 and len(k2) > 6:
                matched = idx_2[k2]
                prefix2_count += 1

        if matched:
            mapping[n["id"]] = {
                "page_index": matched["page"],
                "entry_num": matched["num"],
                "book_name": BOOK_NAME,
            }

    # Save
    with open(MAPPING_FILE, "w", encoding="utf-8") as f:
        json.dump(mapping, f, ensure_ascii=False, indent=2)

    total = exact_count + prefix3_count + prefix2_count
    print(f"\n=== Results ===")
    print(f"  Exact full name:       {exact_count:>5}")
    print(f"  Prefix 3 words:        {prefix3_count:>5}")
    print(f"  Prefix 2 words:        {prefix2_count:>5}")
    print(f"  Total matched:         {total:>5} / {len(our)}")
    print(f"  Coverage:              {total/len(our)*100:.1f}%")
    print(f"  Saved to {MAPPING_FILE}")

    # Spot checks
    print(f"\nSample mappings:")
    count = 0
    for nid, entry in list(mapping.items())[:10]:
        n = narrators[nid]
        print(f"  {nid}: {n.get('name', '?')[:30]:30} -> entry #{entry['entry_num']} pg={entry['page_index']}")
        count += 1


if __name__ == "__main__":
    main()
