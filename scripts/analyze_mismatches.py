#!/usr/bin/env python3
"""Analyze name and kunya mismatches between sanadset.csv and ar_sanad_narrators.csv."""

import csv
import ast
import re
import sys
from collections import Counter, defaultdict

csv.field_size_limit(sys.maxsize)

# ── helpers ──────────────────────────────────────────────────────────────────

def strip_diacritics(text: str) -> str:
    """Remove Arabic diacritics (tashkeel) from text."""
    diacritics = re.compile(r'[\u0610-\u061A\u064B-\u065F\u0670\u06D6-\u06ED]')
    return diacritics.sub('', text).strip()

def normalize(text: str) -> str:
    """Normalize Arabic text: strip diacritics, unify alef/taa/yaa, collapse whitespace."""
    t = strip_diacritics(text)
    # Normalize alef variants → ا
    t = re.sub(r'[إأآٱ]', 'ا', t)
    # Normalize taa marbuta → ة  (sometimes ه is used)
    # Normalize alef maqsura ى → ي
    t = t.replace('ى', 'ي')
    # Collapse whitespace
    t = re.sub(r'\s+', ' ', t).strip()
    return t

def parse_list_field(val: str):
    """Parse a Python-list-like string field."""
    if not val or val in ('', '-', 'nan', 'None'):
        return []
    try:
        return ast.literal_eval(val)
    except:
        return []

# ── load narrator database ───────────────────────────────────────────────────

print("Loading ar_sanad_narrators.csv …")
narrators = {}  # id → row dict
name_to_ids = defaultdict(set)       # normalized name → set of narrator ids
kunia_to_ids = defaultdict(set)       # normalized kunia → set of narrator ids
naming_to_ids = defaultdict(set)      # normalized naming → set of narrator ids

all_known_forms = set()  # all normalized name forms in the narrator DB

with open("data/ar_sanad_narrators.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        nid = int(row['id'])
        narrators[nid] = row

        # primary name
        pname = normalize(row['name'])
        name_to_ids[pname].add(nid)
        all_known_forms.add(pname)

        # kunia
        kunia = row.get('kunia', '').strip()
        if kunia and kunia != '-':
            nk = normalize(kunia)
            kunia_to_ids[nk].add(nid)
            all_known_forms.add(nk)

        # namings (aliases)
        namings = parse_list_field(row.get('namings', ''))
        for alias in namings:
            na = normalize(alias)
            naming_to_ids[na].add(nid)
            all_known_forms.add(na)

print(f"  Loaded {len(narrators)} narrators, {len(all_known_forms)} unique normalized name forms")

# ── extract narrator names from sanadset ─────────────────────────────────────

print("\nLoading sanadset.csv and extracting narrator names …")
sanad_names_raw = Counter()   # raw (with diacritics) name → count
sanad_names_norm = {}         # normalized → set of raw forms

row_count = 0
no_sanad_count = 0

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        row_count += 1
        sanad_str = row.get('Sanad', '')
        if not sanad_str or sanad_str == 'No SANAD':
            no_sanad_count += 1
            continue
        names = parse_list_field(sanad_str)
        for name in names:
            name = name.strip()
            if not name:
                continue
            sanad_names_raw[name] += 1
            norm = normalize(name)
            if norm not in sanad_names_norm:
                sanad_names_norm[norm] = set()
            sanad_names_norm[norm].add(name)

print(f"  Total hadith rows: {row_count}")
print(f"  Rows without sanad: {no_sanad_count}")
print(f"  Unique raw narrator mentions: {len(sanad_names_raw)}")
print(f"  Unique normalized narrator forms: {len(sanad_names_norm)}")

# ── classify each sanadset name ──────────────────────────────────────────────

print("\nClassifying narrator names …")

exact_match = {}       # norm → matched narrator ids
kunia_only = {}        # norm → matched via kunia
naming_match = {}      # norm → matched via namings/aliases
unmatched = {}         # norm → raw forms

for norm, raw_forms in sanad_names_norm.items():
    total_occ = sum(sanad_names_raw[r] for r in raw_forms)

    # Try primary name
    if norm in name_to_ids:
        exact_match[norm] = (name_to_ids[norm], total_occ, raw_forms)
        continue

    # Try namings/aliases
    if norm in naming_to_ids:
        naming_match[norm] = (naming_to_ids[norm], total_occ, raw_forms)
        continue

    # Try kunia
    if norm in kunia_to_ids:
        kunia_only[norm] = (kunia_to_ids[norm], total_occ, raw_forms)
        continue

    unmatched[norm] = (total_occ, raw_forms)

print(f"  Exact name match: {len(exact_match)}")
print(f"  Matched via namings/aliases: {len(naming_match)}")
print(f"  Matched via kunia only: {len(kunia_only)}")
print(f"  UNMATCHED: {len(unmatched)}")

# ── Deeper analysis of unmatched ─────────────────────────────────────────────

print("\n" + "="*80)
print("DETAILED MISMATCH ANALYSIS")
print("="*80)

# 1. Categorize unmatched names
is_kunia_pattern = re.compile(r'^ابو\s|^ابي\s|^ام\s')
contains_ibn = re.compile(r'\bبن\b|\bابن\b')

unmatched_kunia_style = {}
unmatched_partial = {}
unmatched_other = {}

for norm, (occ, raw_forms) in unmatched.items():
    if is_kunia_pattern.match(norm):
        unmatched_kunia_style[norm] = (occ, raw_forms)
    else:
        unmatched_other[norm] = (occ, raw_forms)

print(f"\n── Unmatched names that look like kunya (أبو/أبي/أم ...): {len(unmatched_kunia_style)}")
# Sort by occurrence count descending
for norm, (occ, raw_forms) in sorted(unmatched_kunia_style.items(), key=lambda x: -x[1][0])[:50]:
    raw_sample = list(raw_forms)[0]
    # Try partial matching: check if any known form starts with or contains this
    partial_hits = []
    for known in all_known_forms:
        if norm in known or known in norm:
            partial_hits.append(known)
    partial_str = ""
    if partial_hits:
        partial_str = f"  → possible matches: {partial_hits[:5]}"
    print(f"  [{occ:>6}x] {raw_sample}  (norm: {norm}){partial_str}")

print(f"\n── Unmatched non-kunya names: {len(unmatched_other)}")
for norm, (occ, raw_forms) in sorted(unmatched_other.items(), key=lambda x: -x[1][0])[:50]:
    raw_sample = list(raw_forms)[0]
    # Try to find close matches
    partial_hits = []
    # Check if first word matches any known name's first word
    first_word = norm.split()[0] if norm.split() else ''
    if len(first_word) > 2:
        for known in all_known_forms:
            kfirst = known.split()[0] if known.split() else ''
            if first_word == kfirst and known != norm:
                partial_hits.append(known)
    partial_str = ""
    if partial_hits:
        partial_str = f"  → similar: {partial_hits[:5]}"
    print(f"  [{occ:>6}x] {raw_sample}  (norm: {norm}){partial_str}")

# ── Kunya-vs-name conflicts ─────────────────────────────────────────────────

print(f"\n{'='*80}")
print("KUNYA / NAME INCONSISTENCIES IN NARRATOR DATABASE")
print("="*80)

# Find cases where the same string appears as both a name and a kunia for different narrators
kunia_name_overlap = set(kunia_to_ids.keys()) & set(name_to_ids.keys())
print(f"\n── Strings that are both a primary name AND a kunia for different narrators: {len(kunia_name_overlap)}")
for overlap in sorted(kunia_name_overlap)[:30]:
    name_ids = name_to_ids[overlap]
    k_ids = kunia_to_ids[overlap]
    if name_ids != k_ids:
        print(f"  '{overlap}'")
        for nid in name_ids:
            print(f"    as NAME  → id={nid}: {narrators[nid]['name']}, kunia={narrators[nid].get('kunia','-')}")
        for nid in k_ids:
            print(f"    as KUNIA → id={nid}: {narrators[nid]['name']}, kunia={narrators[nid].get('kunia','-')}")

# ── Multiple raw forms mapping to same normalized ────────────────────────────

print(f"\n{'='*80}")
print("DIACRITICS / SPELLING VARIANTS (same normalized form, multiple raw spellings)")
print("="*80)

multi_raw = {norm: raws for norm, raws in sanad_names_norm.items() if len(raws) > 1}
print(f"\n── Normalized forms with multiple diacritized spellings: {len(multi_raw)}")
for norm, raws in sorted(multi_raw.items(), key=lambda x: -sum(sanad_names_raw[r] for r in x[1]))[:40]:
    total = sum(sanad_names_raw[r] for r in raws)
    print(f"\n  [{total:>6}x] normalized: {norm}")
    for r in sorted(raws, key=lambda x: -sanad_names_raw[x]):
        print(f"           {sanad_names_raw[r]:>6}x  {r}")

# ── Names that match kunia but not primary name ──────────────────────────────

print(f"\n{'='*80}")
print("SANAD NAMES MATCHING KUNIA INSTEAD OF PRIMARY NAME")
print("="*80)
print("(These are names in sanadset that match a narrator's kunia but NOT their primary name)")
print(f"Total: {len(kunia_only)}")
for norm, (ids, occ, raw_forms) in sorted(kunia_only.items(), key=lambda x: -x[1][1])[:30]:
    raw_sample = list(raw_forms)[0]
    print(f"\n  [{occ:>6}x] sanad: {raw_sample}")
    for nid in ids:
        print(f"           → id={nid}: name={narrators[nid]['name']}, kunia={narrators[nid].get('kunia','-')}")

# ── Summary stats ────────────────────────────────────────────────────────────

print(f"\n{'='*80}")
print("SUMMARY")
print("="*80)

total_mentions = sum(sanad_names_raw.values())
exact_mentions = sum(occ for _, occ, _ in exact_match.values())
naming_mentions = sum(occ for _, occ, _ in naming_match.values())
kunia_mentions = sum(occ for _, occ, _ in kunia_only.values())
unmatched_mentions = sum(occ for occ, _ in unmatched.values())

print(f"  Total narrator mentions in sanadset: {total_mentions:>10}")
print(f"  Matched by primary name:             {exact_mentions:>10} ({100*exact_mentions/total_mentions:.1f}%)")
print(f"  Matched by namings/aliases:           {naming_mentions:>10} ({100*naming_mentions/total_mentions:.1f}%)")
print(f"  Matched by kunia only:                {kunia_mentions:>10} ({100*kunia_mentions/total_mentions:.1f}%)")
print(f"  UNMATCHED:                            {unmatched_mentions:>10} ({100*unmatched_mentions/total_mentions:.1f}%)")
print()
print(f"  Unique forms in sanadset: {len(sanad_names_norm)}")
print(f"  Matched:   {len(exact_match) + len(naming_match) + len(kunia_only)}")
print(f"  Unmatched: {len(unmatched)}")
print(f"  Multi-spelling variants: {len(multi_raw)}")
