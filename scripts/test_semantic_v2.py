#!/usr/bin/env python3
"""Parse SemanticHadith TTL with rdflib and match against sanadset.csv"""

import csv, sys, ast, re
from collections import defaultdict
from rdflib import Graph, Namespace, URIRef

csv.field_size_limit(sys.maxsize)

def strip_diacritics(text):
    return re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', text).strip()

SH = Namespace("http://www.semantichadith.com/ontology/")

print("Loading TTL into rdflib (this takes a few minutes for 200MB)...")
g = Graph()
g.parse("/tmp/SemanticHadithKGV2.ttl", format="turtle")
print(f"  Loaded {len(g)} triples")

# ── Extract narrators ───────────────────────────────────────────────────────

print("\nExtracting narrators...")
narrator_names = {}  # HN_id_str → popularName
for s, p, o in g.triples((None, SH.popularName, None)):
    hn = str(s).split('/')[-1]  # e.g. "HN04698"
    narrator_names[hn] = str(o)

narrator_full = {}
for s, p, o in g.triples((None, SH.name, None)):
    hn = str(s).split('/')[-1]
    if hn.startswith('HN'):
        narrator_full[hn] = str(o).replace('@ar', '')

print(f"  {len(narrator_names)} popular names, {len(narrator_full)} full names")

# ── Extract hadith→chain→segments→narrators ─────────────────────────────────

print("\nExtracting chains...")

# hadith → chain
hadith_chain = {}
for s, p, o in g.triples((None, SH.hasNarratorChain, None)):
    hid = str(s).split('/')[-1]
    cid = str(o).split('/')[-1]
    hadith_chain[hid] = cid

# chain → segments
chain_segs = defaultdict(set)
for s, p, o in g.triples((None, SH.hasNarratorSegment, None)):
    cid = str(s).split('/')[-1]
    sid = str(o).split('/')[-1]
    chain_segs[cid].add(sid)

# root segments
for s, p, o in g.triples((None, SH.hasRootNarratorSegment, None)):
    cid = str(s).split('/')[-1]
    sid = str(o).split('/')[-1]
    chain_segs[cid].add(sid)  # make sure root is included

# segment → narrator
seg_narrator = {}
for s, p, o in g.triples((None, SH.refersToNarrator, None)):
    sid = str(s).split('/')[-1]
    hn = str(o).split('/')[-1]
    seg_narrator[sid] = hn

# segment follows
seg_follows = {}
for s, p, o in g.triples((None, SH.follows, None)):
    sid = str(s).split('/')[-1]
    nxt = str(o).split('/')[-1]
    seg_follows[sid] = nxt

print(f"  {len(hadith_chain)} hadiths with chains")
print(f"  {len(seg_narrator)} segments with narrators")

# ── Reconstruct ordered chains ──────────────────────────────────────────────

print("\nReconstructing ordered chains...")

book_map = {
    'SB': 'صحيح البخاري',
    'SM': 'صحيح مسلم',
    'SD': 'سنن أبي داود',
    'JT': 'سنن الترمذي',
    'SN': 'سنن النسائى الصغرى',
    'IM': 'سنن ابن ماجه',
}

hadith_narrators = {}  # (prefix, num) → [narrator_name, ...]

for hid, cid in hadith_chain.items():
    # Extract prefix and number: SB-HD0001 → SB, 1
    m = re.match(r'(SB|SM|SD|JT|SN|IM)-HD(\d+)', hid)
    if not m:
        continue
    prefix = m.group(1)
    num = int(m.group(2))

    segs = chain_segs.get(cid, set())
    if not segs:
        continue

    # Find head: segment not followed by anyone
    all_followed = set(seg_follows.get(s) for s in segs if seg_follows.get(s))
    heads = [s for s in segs if s not in all_followed]
    if not heads:
        continue

    # Walk chain
    ordered = []
    current = heads[0]
    visited = set()
    while current and current not in visited:
        visited.add(current)
        hn = seg_narrator.get(current)
        if hn:
            name = narrator_names.get(hn, narrator_full.get(hn, hn))
            ordered.append((hn, name))
        current = seg_follows.get(current)

    if ordered:
        hadith_narrators[(prefix, num)] = ordered

print(f"  {len(hadith_narrators)} ordered chains")

# Show samples
print("\n  Samples:")
for (prefix, num), chain in list(hadith_narrators.items())[:5]:
    book = book_map.get(prefix, prefix)
    names = [f"{name}({hn})" for hn, name in chain]
    print(f"    {book} #{num}: {' → '.join(names)}")

# ── Match against sanadset ──────────────────────────────────────────────────

print("\n" + "="*80)
print("MATCHING against sanadset.csv (by book + hadith number)")
print("="*80)

sanadset = {}  # (book, num) → [stripped names]
target_books = set(book_map.values())

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        book = row.get('Book', '')
        if book not in target_books:
            continue
        num = int(row.get('Num_hadith', 0) or 0)
        if num == 0:
            continue
        sanad = row.get('Sanad', '')
        if not sanad or sanad == 'No SANAD':
            continue
        try:
            names = ast.literal_eval(sanad)
            stripped = [strip_diacritics(n.strip()) for n in names if n.strip()]
            sanadset[(book, num)] = stripped
        except:
            continue

print(f"  Sanadset chains for 6 books: {len(sanadset)}")

# Match
matched_num = 0
not_found = 0
same_length = 0
diff_length = 0
name_matches = 0
total_names_compared = 0

per_book = defaultdict(lambda: {'sem': 0, 'matched': 0, 'same_len': 0, 'name_match': 0, 'names_total': 0})

samples = []

for (prefix, num), sem_chain in hadith_narrators.items():
    book_ar = book_map.get(prefix)
    if not book_ar:
        continue

    per_book[book_ar]['sem'] += 1
    san_chain = sanadset.get((book_ar, num))
    if san_chain is None:
        not_found += 1
        continue

    matched_num += 1
    per_book[book_ar]['matched'] += 1

    sem_names = [strip_diacritics(name) for _, name in sem_chain]

    if len(sem_names) == len(san_chain):
        same_length += 1
        per_book[book_ar]['same_len'] += 1
        # Compare name by name
        for sn, san in zip(sem_names, san_chain):
            total_names_compared += 1
            per_book[book_ar]['names_total'] += 1
            if sn == san or sn in san or san in sn:
                name_matches += 1
                per_book[book_ar]['name_match'] += 1
    else:
        diff_length += 1
        if len(samples) < 5:
            samples.append((book_ar, num, sem_names, san_chain))

print(f"\n  SemanticHadith total chains: {len(hadith_narrators)}")
print(f"  Matched by book+number:     {matched_num} ({100*matched_num/len(hadith_narrators):.1f}%)")
print(f"  Not found in sanadset:      {not_found}")
print(f"  Same chain length:          {same_length} ({100*same_length/max(matched_num,1):.1f}%)")
print(f"  Different chain length:     {diff_length}")
print(f"  Name matches (substring):   {name_matches}/{total_names_compared} ({100*name_matches/max(total_names_compared,1):.1f}%)")

print(f"\n  Per-book breakdown:")
print(f"  {'Book':<30} {'In KG':>6} {'Matched':>8} {'SameLen':>8} {'NameMatch':>10}")
for book_ar in sorted(per_book.keys()):
    b = per_book[book_ar]
    print(f"  {book_ar:<30} {b['sem']:>6} {b['matched']:>8} {b['same_len']:>8} {b['name_match']}/{b['names_total']}")

if samples:
    print(f"\n  LENGTH MISMATCH SAMPLES:")
    for book, num, sem, san in samples:
        print(f"\n    {book} #{num}:")
        print(f"      Semantic ({len(sem)}): {sem}")
        print(f"      Sanadset ({len(san)}): {san}")
