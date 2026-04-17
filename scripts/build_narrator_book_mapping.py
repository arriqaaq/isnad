#!/usr/bin/env python3
"""
Build narrator → Tahdhib al-Tahdhib page mapping.

Matching strategy (in priority order):
  1. Exact full name match (highest confidence)
  2. Prefix-4+ match (4 or more name parts match — high confidence)
  3. Prefix-3 match ONLY if the 3-word prefix is unique in Tahdhib (no ambiguity)
  4. Skip ambiguous matches (better to have no mapping than a wrong one)

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

    # Extract Tahdhib narrator entries
    print("\nExtracting Tahdhib narrator entries...")
    tahdhib_entries = []
    for h in headings:
        m = re.match(r"\[(\d+)\]\s*(?:\([^)]*\)\s*)?(.+)", h["title"])
        if m and h["level"] == 2:
            name = m.group(2).strip()
            tahdhib_entries.append({
                "num": int(m.group(1)),
                "name": name,
                "name_norm": normalize(name),
                "page": h["page"] - 1,
            })
    print(f"  Tahdhib entries: {len(tahdhib_entries)}")

    # Build indices at multiple prefix lengths
    idx_full: dict[str, list] = {}
    idx_4: dict[str, list] = {}
    idx_3: dict[str, list] = {}

    for e in tahdhib_entries:
        k = e["name_norm"]
        idx_full.setdefault(k, []).append(e)

        k4 = name_parts(k, 4)
        if len(k4.split()) >= 4:
            idx_4.setdefault(k4, []).append(e)

        k3 = name_parts(k, 3)
        if len(k3.split()) >= 3:
            idx_3.setdefault(k3, []).append(e)

    # Count ambiguity
    ambiguous_3 = {k for k, v in idx_3.items() if len(v) > 1}
    ambiguous_4 = {k for k, v in idx_4.items() if len(v) > 1}
    print(f"  Ambiguous 3-word prefixes: {len(ambiguous_3)} (e.g. عبد الله بن = {len(idx_3.get(normalize('عبد الله بن'), []))} entries)")
    print(f"  Ambiguous 4-word prefixes: {len(ambiguous_4)}")

    # Build narrator list
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

    # Match with strict strategy
    print("\nMatching (strict mode — no ambiguous matches)...")
    mapping = {}
    exact_count = 0
    prefix4_count = 0
    prefix3_unique_count = 0
    skipped_ambiguous = 0

    for n in our:
        matched = None
        method = ""

        # 1. Exact full name
        for name_to_try in [n["name_norm"], n["popular_norm"]]:
            if not name_to_try:
                continue
            candidates = idx_full.get(name_to_try, [])
            if len(candidates) == 1:
                matched = candidates[0]
                method = "exact"
                break
            elif len(candidates) > 1:
                # Multiple exact matches — still take first (rare, same name different people)
                matched = candidates[0]
                method = "exact"
                break

        # 2. Prefix-4 match (high confidence)
        if not matched:
            for name_to_try in [n["name_norm"], n["popular_norm"]]:
                if not name_to_try:
                    continue
                k4 = name_parts(name_to_try, 4)
                if len(k4.split()) >= 4 and k4 in idx_4:
                    candidates = idx_4[k4]
                    if len(candidates) == 1:
                        matched = candidates[0]
                        method = "prefix4_unique"
                        break
                    else:
                        # Multiple matches at prefix-4 — try prefix-5
                        k5 = name_parts(name_to_try, 5)
                        for c in candidates:
                            if name_parts(c["name_norm"], 5) == k5 and len(k5.split()) >= 5:
                                matched = c
                                method = "prefix5"
                                break
                        if matched:
                            break
                        # Still ambiguous — take first but only if reasonable
                        matched = candidates[0]
                        method = "prefix4_first"
                        break

        # 3. Prefix-3 match ONLY if unambiguous
        if not matched:
            for name_to_try in [n["name_norm"], n["popular_norm"]]:
                if not name_to_try:
                    continue
                k3 = name_parts(name_to_try, 3)
                if len(k3.split()) >= 3 and k3 in idx_3:
                    candidates = idx_3[k3]
                    if len(candidates) == 1:
                        # Unique 3-word prefix — safe match
                        matched = candidates[0]
                        method = "prefix3_unique"
                        break
                    else:
                        skipped_ambiguous += 1
                        # DO NOT match — too ambiguous

        if matched:
            db_id = "hn_" + n["id"].removeprefix("HN")
            mapping[db_id] = {
                "page_index": matched["page"],
                "entry_num": matched["num"],
                "book_name": BOOK_NAME,
            }
            if method == "exact":
                exact_count += 1
            elif method.startswith("prefix4") or method == "prefix5":
                prefix4_count += 1
            elif method == "prefix3_unique":
                prefix3_unique_count += 1

    # Save
    with open(MAPPING_FILE, "w", encoding="utf-8") as f:
        json.dump(mapping, f, ensure_ascii=False, indent=2)

    total = exact_count + prefix4_count + prefix3_unique_count
    print(f"\n=== Results (strict matching) ===")
    print(f"  Exact full name:           {exact_count:>5}")
    print(f"  Prefix 4+ words:           {prefix4_count:>5}")
    print(f"  Prefix 3 (unique only):    {prefix3_unique_count:>5}")
    print(f"  Total matched:             {total:>5} / {len(our)}")
    print(f"  Skipped (ambiguous):       {skipped_ambiguous:>5}")
    print(f"  Coverage:                  {total/len(our)*100:.1f}%")
    print(f"  Saved to {MAPPING_FILE}")

    # Spot checks
    print(f"\nSample mappings:")
    for db_id, entry in list(mapping.items())[:10]:
        src_id = "HN" + db_id.removeprefix("hn_")
        n = narrators.get(src_id, {})
        print(f"  {db_id}: {n.get('name', '?')[:30]:30} -> entry #{entry['entry_num']} pg={entry['page_index']}")

    # Verify the previously-wrong narrator is now unmapped
    print(f"\n=== Verification ===")
    print(f"  hn_05049 mapped: {'hn_05049' in mapping}")
    if "hn_05049" in mapping:
        e = mapping["hn_05049"]
        print(f"    entry #{e['entry_num']} pg={e['page_index']}")


if __name__ == "__main__":
    main()
