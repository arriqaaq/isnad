#!/usr/bin/env python3
"""
Simulate the Rust NameResolver logic in Python to verify match rates.
This mirrors the exact normalization and matching strategy from:
  - src/ingest/sanadset.rs: normalize_arabic() + normalize_kunya()
  - src/ingest/name_resolver.rs: NameResolver
"""

import csv
import ast
import re
import sys
from collections import Counter, defaultdict

csv.field_size_limit(sys.maxsize)

# ── Normalization (mirrors Rust normalize_arabic + normalize_kunya) ──────────

def strip_diacritics(text: str) -> str:
    diacritics = re.compile(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]')
    return diacritics.sub('', text)

def normalize_arabic(text: str) -> str:
    out = []
    for c in text:
        code = ord(c)
        # Skip diacritics
        if (0x064B <= code <= 0x065F) or code == 0x0670 or code == 0x0640 or (0x0610 <= code <= 0x061A):
            continue
        # Alef variants → bare alef
        if c in 'أإآٱ':
            out.append('ا')
        # Taa marbuta → haa
        elif c == 'ة':
            out.append('ه')
        # Alef maqsura → yaa
        elif c == 'ى':
            out.append('ي')
        # Keep Arabic letters and spaces
        elif (0x0620 <= code <= 0x064A) or c == ' ':
            out.append(c)
    collapsed = ' '.join(''.join(out).split())
    return normalize_kunya(collapsed)

def normalize_kunya(text: str) -> str:
    words = text.split(' ')
    if not words:
        return text
    result = []
    i = 0
    while i < len(words):
        w = words[i]
        if i + 1 < len(words):
            if w in ('ابي', 'ابى'):
                result.append('ابو')
                i += 1
                continue
            if w == 'امي':
                result.append('ام')
                i += 1
                continue
        result.append(w)
        i += 1
    return ' '.join(result)

def parse_list_field(val: str):
    if not val or val in ('', '-', 'nan', 'None'):
        return []
    try:
        return ast.literal_eval(val)
    except:
        return []

def parse_id_list(val: str):
    if not val or val in ('', '-', 'nan', 'None'):
        return []
    try:
        return [int(x) for x in ast.literal_eval(val)]
    except:
        return []

# ── Load narrator database ──────────────────────────────────────────────────

print("Loading ar_sanad_narrators.csv ...")
narrators = {}
unified_index = defaultdict(set)  # normalized → set of IDs

with open("data/ar_sanad_narrators.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        nid = int(row['id'])
        narrators[nid] = {
            'name': row['name'],
            'narrated_from': parse_id_list(row.get('narrated_from', '')),
            'narrated_to': parse_id_list(row.get('narrated_to', '')),
        }

        # Primary name
        norm = normalize_arabic(row['name'])
        if norm:
            unified_index[norm].add(nid)

        # Namings/aliases
        for alias in parse_list_field(row.get('namings', '')):
            na = normalize_arabic(alias)
            if na:
                unified_index[na].add(nid)

        # Kunia
        kunia = row.get('kunia', '').strip()
        if kunia and kunia != '-':
            nk = normalize_arabic(kunia)
            if nk:
                unified_index[nk].add(nid)

print(f"  {len(narrators)} narrators, {len(unified_index)} unified forms")

# ── Resolution functions ────────────────────────────────────────────────────

def resolve_name(normalized):
    """Returns (status, ids)"""
    ids = unified_index.get(normalized)
    if ids and len(ids) == 1:
        return 'resolved', list(ids)
    if ids and len(ids) > 1:
        return 'ambiguous', list(ids)
    return 'unresolved', []

def disambiguate(candidates, prev_id, next_id):
    if prev_id is None and next_id is None:
        return candidates
    scored = []
    for cid in candidates:
        score = 0
        rec = narrators.get(cid)
        if rec:
            if prev_id is not None and prev_id in rec['narrated_to']:
                score += 2
            if next_id is not None and next_id in rec['narrated_from']:
                score += 2
        scored.append((cid, score))
    max_score = max(s for _, s in scored) if scored else 0
    if max_score == 0:
        return candidates
    return [cid for cid, s in scored if s == max_score]

def resolve_partial(normalized, prev_id, next_id):
    if len(normalized) < 4:
        return 'unresolved', []
    candidates = set()
    for form, ids in unified_index.items():
        if form.startswith(normalized) or normalized.startswith(form):
            candidates.update(ids)
    if not candidates:
        return 'unresolved', []
    if len(candidates) == 1:
        return 'resolved', list(candidates)
    filtered = disambiguate(list(candidates), prev_id, next_id)
    if len(filtered) == 1:
        return 'resolved', filtered
    return 'ambiguous', filtered

def resolve_chain(raw_names):
    """Multi-pass chain resolution. Returns list of (raw, status, ids)."""
    normalized = [normalize_arabic(n) for n in raw_names]

    # Skip relative references
    skip = {'ابي', 'ابيه', 'جده', 'عمه', 'امه', 'اخيه', 'ابيها'}

    # Pass 1
    results = []
    for norm in normalized:
        if norm in skip:
            results.append(('skip', []))
        else:
            results.append(resolve_name(norm))

    # Pass 2 & 3: context disambiguation
    for _ in range(2):
        changed = False
        for i in range(len(results)):
            if results[i][0] == 'resolved':
                continue
            if results[i][0] == 'skip':
                continue

            prev_id = results[i-1][1][0] if i > 0 and results[i-1][0] == 'resolved' else None
            next_id = results[i+1][1][0] if i+1 < len(results) and results[i+1][0] == 'resolved' else None

            status, ids = results[i]
            if status == 'ambiguous':
                filtered = disambiguate(ids, prev_id, next_id)
                if len(filtered) == 1:
                    results[i] = ('resolved', filtered)
                    changed = True
            elif status == 'unresolved':
                s, fids = resolve_partial(normalized[i], prev_id, next_id)
                if s == 'resolved':
                    results[i] = (s, fids)
                    changed = True
        if not changed:
            break

    return [(raw, r[0], r[1]) for raw, r in zip(raw_names, results)]

# ── Process sanadset ────────────────────────────────────────────────────────

print("\nProcessing sanadset.csv ...")

total_mentions = 0
resolved_mentions = 0
ambiguous_mentions = 0
unresolved_mentions = 0
skipped_mentions = 0
no_sanad = 0
row_count = 0

unresolved_names = Counter()
ambiguous_names = Counter()

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        row_count += 1
        sanad_str = row.get('Sanad', '')
        if not sanad_str or sanad_str == 'No SANAD':
            no_sanad += 1
            continue
        names = parse_list_field(sanad_str)
        if not names:
            continue

        chain_results = resolve_chain(names)
        for raw, status, ids in chain_results:
            total_mentions += 1
            if status == 'resolved':
                resolved_mentions += 1
            elif status == 'ambiguous':
                ambiguous_mentions += 1
                ambiguous_names[normalize_arabic(raw)] += 1
            elif status == 'skip':
                skipped_mentions += 1
            else:
                unresolved_mentions += 1
                unresolved_names[normalize_arabic(raw)] += 1

        if row_count % 100000 == 0:
            print(f"  ... {row_count} rows processed")

print(f"\n{'='*80}")
print("VERIFICATION RESULTS")
print(f"{'='*80}")
print(f"  Total hadith rows:    {row_count}")
print(f"  Rows without sanad:   {no_sanad}")
print(f"  Total narrator mentions: {total_mentions}")
print(f"  Skipped (relatives):     {skipped_mentions}")
useful = total_mentions - skipped_mentions
print(f"  Useful mentions:         {useful}")
print()
print(f"  RESOLVED:    {resolved_mentions:>10} ({100*resolved_mentions/useful:.1f}% of useful)")
print(f"  AMBIGUOUS:   {ambiguous_mentions:>10} ({100*ambiguous_mentions/useful:.1f}% of useful)")
print(f"  UNRESOLVED:  {unresolved_mentions:>10} ({100*unresolved_mentions/useful:.1f}% of useful)")

print(f"\n── Top 30 UNRESOLVED names (by frequency) ──")
for norm, count in unresolved_names.most_common(30):
    print(f"  [{count:>6}x] {norm}")

print(f"\n── Top 30 AMBIGUOUS names (by frequency) ──")
for norm, count in ambiguous_names.most_common(30):
    ids = list(unified_index.get(norm, set()))
    print(f"  [{count:>6}x] {norm}  ({len(ids)} candidates: {ids[:5]})")

print(f"\n── Total unique unresolved: {len(unresolved_names)}")
print(f"── Total unique ambiguous:  {len(ambiguous_names)}")
