#!/usr/bin/env python3
"""Verify the extracted semantic_hadith.json data integrity."""

import json
import sys
from collections import Counter

DATA_PATH = "data/semantic_hadith.json"

EXPECTED_BOOKS = {"SB", "SM", "SD", "JT", "SN", "IM"}
BOOK_NAMES = {
    "SB": "صحيح البخاري",
    "SM": "صحيح مسلم",
    "SD": "سنن أبي داود",
    "JT": "جامع الترمذي",
    "SN": "سنن النسائى الصغرى",
    "IM": "سنن ابن ماجه",
}

def main():
    print(f"Loading {DATA_PATH}...")
    with open(DATA_PATH, encoding="utf-8") as f:
        data = json.load(f)

    narrators = data["narrators"]
    hadiths = data["hadiths"]

    errors = []
    warnings = []

    # ── Narrator checks ─────────────────────────────────────────────────────

    print(f"\n=== NARRATORS: {len(narrators)} ===")

    narrator_fields = Counter()
    for hn, rec in narrators.items():
        if not hn.startswith("HN"):
            errors.append(f"Invalid narrator ID: {hn}")
        for field in ["popularName", "name", "generation", "lineage", "residence",
                       "deathYear", "birthYear", "teknonym", "title", "office", "attribute"]:
            if field in rec:
                narrator_fields[field] += 1

    print("  Field coverage:")
    for field, count in sorted(narrator_fields.items(), key=lambda x: -x[1]):
        pct = 100 * count / len(narrators)
        print(f"    {field:<15} {count:>5} ({pct:.0f}%)")

    no_name = sum(1 for r in narrators.values() if "popularName" not in r and "name" not in r)
    if no_name:
        warnings.append(f"{no_name} narrators have no name (ID-only entries in SemanticHadith KG)")

    # ── Hadith checks ───────────────────────────────────────────────────────

    print(f"\n=== HADITHS: {len(hadiths)} ===")

    books = Counter()
    no_chain = 0
    no_ref = 0
    no_ar = 0
    no_en = 0
    invalid_chain_refs = 0
    total_chain_narrators = 0

    for hid, rec in hadiths.items():
        book = rec.get("book", "")
        books[book] += 1

        if not rec.get("chain"):
            no_chain += 1
        else:
            for hn in rec["chain"]:
                total_chain_narrators += 1
                if hn not in narrators:
                    invalid_chain_refs += 1

        if not rec.get("refNo"):
            no_ref += 1
        if not rec.get("textAr"):
            no_ar += 1
        if not rec.get("textEn"):
            no_en += 1

    print("  Per-book counts:")
    for prefix in sorted(books):
        name = BOOK_NAMES.get(prefix, prefix)
        print(f"    {name}: {books[prefix]}")

    missing_books = EXPECTED_BOOKS - set(books.keys())
    if missing_books:
        errors.append(f"Missing books: {missing_books}")

    if no_chain:
        errors.append(f"{no_chain} hadiths have no narrator chain")
    if no_ref:
        warnings.append(f"{no_ref} hadiths missing reference number")
    if no_ar:
        errors.append(f"{no_ar} hadiths missing Arabic text")
    if invalid_chain_refs:
        errors.append(f"{invalid_chain_refs} chain narrator refs point to unknown HN IDs")

    print(f"\n  Chain narrators: {total_chain_narrators} total references")
    print(f"  Arabic text:  {len(hadiths) - no_ar}/{len(hadiths)}")
    print(f"  English text: {len(hadiths) - no_en}/{len(hadiths)}")

    # Enrichment data coverage
    has_type = sum(1 for h in hadiths.values() if "type" in h)
    has_topics = sum(1 for h in hadiths.values() if "topics" in h)
    has_mentions = sum(1 for h in hadiths.values() if "mentions" in h)
    has_verses = sum(1 for h in hadiths.values() if "quranVerses" in h)
    has_similar = sum(1 for h in hadiths.values() if "similar" in h)
    has_strong = sum(1 for h in hadiths.values() if "stronglySimilar" in h)
    has_seealso = sum(1 for h in hadiths.values() if "seeAlso" in h)
    has_chapter = sum(1 for h in hadiths.values() if "chapter" in h)

    print(f"\n  Enrichment coverage:")
    print(f"    hadithType:       {has_type:>6} ({100*has_type/len(hadiths):.0f}%)")
    print(f"    topics:           {has_topics:>6} ({100*has_topics/len(hadiths):.0f}%)")
    print(f"    mentions:         {has_mentions:>6} ({100*has_mentions/len(hadiths):.0f}%)")
    print(f"    quranVerses:      {has_verses:>6} ({100*has_verses/len(hadiths):.0f}%)")
    print(f"    similar:          {has_similar:>6} ({100*has_similar/len(hadiths):.0f}%)")
    print(f"    stronglySimilar:  {has_strong:>6} ({100*has_strong/len(hadiths):.0f}%)")
    print(f"    seeAlso:          {has_seealso:>6} ({100*has_seealso/len(hadiths):.0f}%)")
    print(f"    chapter:          {has_chapter:>6} ({100*has_chapter/len(hadiths):.0f}%)")

    # ── Unique narrators in chains vs narrator DB ───────────────────────────

    chain_hn_ids = set()
    for h in hadiths.values():
        chain_hn_ids.update(h.get("chain", []))

    in_both = chain_hn_ids & set(narrators.keys())
    in_chains_only = chain_hn_ids - set(narrators.keys())
    in_db_only = set(narrators.keys()) - chain_hn_ids

    print(f"\n  Narrator coverage:")
    print(f"    In chains AND narrator DB: {len(in_both)}")
    print(f"    In chains only (no bio):   {len(in_chains_only)}")
    print(f"    In DB only (not in chains): {len(in_db_only)}")

    if in_chains_only:
        warnings.append(f"{len(in_chains_only)} narrators in chains but not in narrator DB")

    # ── Report ──────────────────────────────────────────────────────────────

    print(f"\n{'='*60}")
    if errors:
        print(f"ERRORS ({len(errors)}):")
        for e in errors:
            print(f"  ✗ {e}")
    if warnings:
        print(f"WARNINGS ({len(warnings)}):")
        for w in warnings:
            print(f"  ⚠ {w}")
    if not errors:
        print("✓ ALL CHECKS PASSED")
    else:
        print(f"\n✗ {len(errors)} errors found")
        sys.exit(1)


if __name__ == "__main__":
    main()
