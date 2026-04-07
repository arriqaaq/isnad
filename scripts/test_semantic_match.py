#!/usr/bin/env python3
"""Test: can we match SemanticHadith KG chains to sanadset.csv rows?
Extract narrator chains from TTL, match against sanadset by hadith number and text."""

import csv, sys, re, ast
from collections import defaultdict

csv.field_size_limit(sys.maxsize)

def strip_diacritics(text):
    return re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', text).strip()

# ── Parse TTL file (targeted extraction, not full RDF parsing) ──────────────

print("Parsing SemanticHadith TTL (targeted extraction)...")

ttl_path = "/tmp/SemanticHadithKGV2.ttl"

# We need:
# 1. Hadith → chain mapping: :SB-HD0001 :hasNarratorChain :SB-HD0001-Chain
# 2. Chain → segments: :SB-HD0001-Chain :hasNarratorSegment :SB-HD0001-ChainSeg-1
# 3. Segment → narrator: :SB-HD0001-ChainSeg-1 :refersToNarrator :HN04698
# 4. Segment ordering: :SB-HD0001-ChainSeg-1 :follows :SB-HD0001-ChainSeg-2
# 5. Root segment: :SB-HD0001-Chain :hasRootNarratorSegment :SB-HD0001-ChainSeg-6
# 6. Narrator names: :HN04698 :popularName "..." ; :name "..."

# Strategy: single pass, extract triples we care about

narrator_names = {}        # HN_id → popular_name
narrator_full = {}         # HN_id → full name
hadith_chain = {}          # hadith_id → chain_id
chain_segments = defaultdict(set)  # chain_id → set of segment_ids
seg_narrator = {}          # segment_id → HN_id
seg_follows = {}           # segment_id → next_segment_id
chain_root = {}            # chain_id → root_segment_id
hadith_ref = {}            # hadith_id → reference number
hadith_text_ar = {}        # hadith_id → arabic text (first 200 chars for matching)

PREFIX = "http://www.semantichadith.com/ontology/"

line_count = 0
with open(ttl_path, encoding="utf-8") as f:
    current_subject = None
    for line in f:
        line_count += 1
        line = line.strip()

        # Track current subject for multi-line entries
        if line and not line.startswith('@') and not line.startswith('#'):
            # Check for new subject (starts without whitespace in original)
            if not line[0] in (' ', '\t', ';', '.') and ':' in line:
                parts = line.split(None, 1)
                if parts:
                    current_subject = parts[0]

        # Narrator popular names
        m = re.search(r':HN(\d+)\s', line)
        if m and ':popularName' in line:
            hn_id = m.group(1)
            nm = re.search(r':popularName\s+"([^"]*)"', line)
            if nm:
                narrator_names[hn_id] = nm.group(1)

        if m and ':name' in line and ':popularName' not in line:
            hn_id = m.group(1)
            nm = re.search(r':name\s+"([^"]*)"', line)
            if nm:
                narrator_full[hn_id] = nm.group(1)

        # Hadith → chain
        m = re.search(r':((?:SB|SM|SD|JT|SN|IM)-HD\d+)\s+:hasNarratorChain\s+:(\S+)', line)
        if m:
            hadith_chain[m.group(1)] = m.group(2)

        # Hadith reference number
        m = re.search(r':((?:SB|SM|SD|JT|SN|IM)-HD\d+)', line)
        if m and 'hadithReferenceNo' in line:
            hid = m.group(1)
            rm = re.search(r'hadithReferenceNo\s+"(\d+)"', line)
            if rm:
                hadith_ref[hid] = int(rm.group(1))

        # Chain → segments
        m = re.search(r':(\S+-Chain)\s+:hasNarratorSegment\s+:(\S+)', line)
        if m:
            chain_segments[m.group(1)].add(m.group(2))

        # Chain root segment
        m = re.search(r':(\S+-Chain)\s+:hasRootNarratorSegment\s+:(\S+)', line)
        if m:
            chain_root[m.group(1)] = m.group(2)

        # Segment → narrator
        m = re.search(r':(\S+ChainSeg-\d+)\s+:refersToNarrator\s+:HN(\d+)', line)
        if m:
            seg_narrator[m.group(1)] = m.group(2)

        # Segment follows
        m = re.search(r':(\S+ChainSeg-\d+)\s+:follows\s+:(\S+ChainSeg-\d+)', line)
        if m:
            seg_follows[m.group(1)] = m.group(2)

        if line_count % 500000 == 0:
            print(f"  ... {line_count} lines")

print(f"  Parsed {line_count} lines")
print(f"  Narrators: {len(narrator_names)} popular names, {len(narrator_full)} full names")
print(f"  Hadiths with chains: {len(hadith_chain)}")
print(f"  Hadiths with ref numbers: {len(hadith_ref)}")

# ── Reconstruct ordered chains ──────────────────────────────────────────────

print("\nReconstructing ordered chains...")

# For each hadith, build ordered narrator list
# The chain is: seg1 follows seg2 follows ... follows root
# So we start from any non-root segment and follow the chain

book_prefix_map = {
    'SB': 'صحيح البخاري',
    'SM': 'صحيح مسلم',
    'SD': 'سنن أبي داود',
    'JT': 'سنن الترمذي',
    'SN': 'سنن النسائى الصغرى',
    'IM': 'سنن ابن ماجه',
}

# Build hadith → ordered narrator names
hadith_narrators = {}  # (book_prefix, ref_no) → [narrator_popular_name, ...]

for hid, cid in hadith_chain.items():
    prefix = hid.split('-')[0]
    ref_no = hadith_ref.get(hid)
    if ref_no is None:
        continue

    segments = chain_segments.get(cid, set())
    if not segments:
        continue

    # Build segment order: find the segment that nobody follows (the first in chain)
    followed_by = {}  # seg → who follows it
    for seg in segments:
        nxt = seg_follows.get(seg)
        if nxt:
            followed_by[nxt] = seg

    # Find head: segment not in followed_by values... actually, find segment
    # not followed by anyone (the first transmitter)
    all_followed = set(seg_follows.get(s) for s in segments if seg_follows.get(s))
    heads = [s for s in segments if s not in all_followed]

    if not heads:
        continue

    # Walk from head following the chain
    ordered = []
    current = heads[0]
    visited = set()
    while current and current not in visited:
        visited.add(current)
        hn_id = seg_narrator.get(current)
        if hn_id:
            name = narrator_names.get(hn_id, narrator_full.get(hn_id, f'HN{hn_id}'))
            ordered.append(name)
        current = seg_follows.get(current)

    if ordered:
        hadith_narrators[(prefix, ref_no)] = ordered

print(f"  Reconstructed {len(hadith_narrators)} ordered chains")

# ── Show samples ────────────────────────────────────────────────────────────

print("\n" + "="*80)
print("SAMPLE CHAINS from SemanticHadith KG")
print("="*80)
for (prefix, ref), names in list(hadith_narrators.items())[:10]:
    book = book_prefix_map.get(prefix, prefix)
    print(f"\n  {book} #{ref}: {' → '.join(names)}")

# ── Match against sanadset ──────────────────────────────────────────────────

print("\n" + "="*80)
print("MATCHING against sanadset.csv")
print("="*80)

# Load sanadset chains for the 6 books, indexed by (book, hadith_num)
print("\nLoading sanadset.csv (6 books only)...")

sanadset_chains = {}  # (book_name, num) → [stripped narrator names]
target_books = set(book_prefix_map.values())

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
            sanadset_chains[(book, num)] = stripped
        except:
            continue

print(f"  Loaded {len(sanadset_chains)} sanadset chains for 6 books")

# Try matching by book + hadith number
matched = 0
chain_match = 0
chain_partial = 0
chain_mismatch = 0
not_in_sanadset = 0

match_details = []

for (prefix, ref), sem_names in hadith_narrators.items():
    book_ar = book_prefix_map.get(prefix)
    if not book_ar:
        continue

    san_names = sanadset_chains.get((book_ar, ref))
    if san_names is None:
        not_in_sanadset += 1
        continue

    matched += 1

    # Compare: strip diacritics from SemanticHadith names too (should already be plain)
    sem_stripped = [strip_diacritics(n) for n in sem_names]

    # Check if chain lengths match
    if len(sem_stripped) == len(san_names):
        # Check narrator-by-narrator
        all_match = all(s in n or n in s for s, n in zip(sem_stripped, san_names))
        if all_match:
            chain_match += 1
        else:
            chain_partial += 1
            if len(match_details) < 10:
                match_details.append((prefix, ref, sem_stripped, san_names))
    else:
        chain_mismatch += 1
        if len(match_details) < 10:
            match_details.append((prefix, ref, sem_stripped, san_names))

print(f"\n  SemanticHadith chains: {len(hadith_narrators)}")
print(f"  Matched by book+number: {matched}")
print(f"  Not in sanadset:        {not_in_sanadset}")
print(f"")
print(f"  Of matched:")
print(f"    Chain fully matches:  {chain_match} ({100*chain_match/max(matched,1):.1f}%)")
print(f"    Chain partial match:  {chain_partial}")
print(f"    Chain length differs: {chain_mismatch}")

# Per-book breakdown
print(f"\n  Per-book match rates:")
for prefix, book_ar in sorted(book_prefix_map.items()):
    total_sem = sum(1 for (p, r) in hadith_narrators if p == prefix)
    matched_book = sum(1 for (p, r) in hadith_narrators if p == prefix and (book_ar, r) in sanadset_chains)
    print(f"    {book_ar}: {matched_book}/{total_sem} ({100*matched_book/max(total_sem,1):.1f}%)")

if match_details:
    print(f"\n  SAMPLE MISMATCHES (first 5):")
    for prefix, ref, sem, san in match_details[:5]:
        book = book_prefix_map.get(prefix, prefix)
        print(f"\n    {book} #{ref}:")
        print(f"      Semantic ({len(sem)}): {sem[:5]}")
        print(f"      Sanadset ({len(san)}): {san[:5]}")
