#!/usr/bin/env python3
"""Dry run: match hadiths by text, align chains positionally, and measure
how accurately we can map sanadset narrator names → SemanticHadith narrator IDs."""

import csv, sys, re, ast
from collections import defaultdict, Counter
from rdflib import Graph, Namespace

csv.field_size_limit(sys.maxsize)

def strip(t):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', t)
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه').replace('ى', 'ي')
    return re.sub(r'[^\u0620-\u064A]', '', t)

def strip_keep_spaces(t):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', t)
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه').replace('ى', 'ي')
    return ' '.join(t.split())

SH = Namespace('http://www.semantichadith.com/ontology/')
print("Loading SemanticHadith KG...")
g = Graph()
g.parse('/tmp/SemanticHadithKGV2.ttl', format='turtle')

# ── Extract narrator info ───────────────────────────────────────────────────
narrator_info = {}
for s in g.subjects(predicate=None, object=SH.HadithNarrator):
    hn = str(s).split('/')[-1]
    if not hn.startswith('HN'):
        continue
    rec = {}
    for p, o in g.predicate_objects(subject=s):
        prop = str(p).split('/')[-1]
        if prop in ('popularName', 'name', 'deathYear', 'generation', 'lineage',
                     'residence', 'teknonym', 'narratorID', 'birthYear'):
            rec[prop] = str(o).replace('@ar', '').split('^^')[0]
    narrator_info[hn] = rec
print(f"  {len(narrator_info)} narrators")

# ── Extract chains ──────────────────────────────────────────────────────────
print("Extracting chains...")
hadith_chain = {}
for s, p, o in g.triples((None, SH.hasNarratorChain, None)):
    hadith_chain[str(s).split('/')[-1]] = str(o).split('/')[-1]

chain_segs = defaultdict(set)
for s, p, o in g.triples((None, SH.hasNarratorSegment, None)):
    chain_segs[str(s).split('/')[-1]].add(str(o).split('/')[-1])
for s, p, o in g.triples((None, SH.hasRootNarratorSegment, None)):
    chain_segs[str(s).split('/')[-1]].add(str(o).split('/')[-1])

seg_narrator = {}
for s, p, o in g.triples((None, SH.refersToNarrator, None)):
    seg_narrator[str(s).split('/')[-1]] = str(o).split('/')[-1]

seg_follows = {}
for s, p, o in g.triples((None, SH.follows, None)):
    sid = str(s).split('/')[-1]
    nxt = str(o).split('/')[-1]
    if 'ChainSeg' in sid:
        seg_follows[sid] = nxt

sem_chains = {}  # hadith_id → [(hn_id, popularName), ...]
for hid, cid in hadith_chain.items():
    segs = chain_segs.get(cid, set())
    if not segs:
        continue
    all_followed = set(seg_follows.get(s) for s in segs if seg_follows.get(s))
    heads = [s for s in segs if s not in all_followed]
    if not heads:
        continue
    ordered = []
    current = heads[0]
    visited = set()
    while current and current not in visited:
        visited.add(current)
        hn = seg_narrator.get(current)
        if hn:
            name = narrator_info.get(hn, {}).get('popularName',
                   narrator_info.get(hn, {}).get('name', hn))
            ordered.append((hn, name))
        current = seg_follows.get(current)
    if ordered:
        sem_chains[hid] = ordered

print(f"  {len(sem_chains)} chains")

# ── Build text index ────────────────────────────────────────────────────────
print("Building text index...")
sh_texts = defaultdict(list)
for s, p, o in g.triples((None, SH.fullHadithText, None)):
    t = str(o)
    hid = str(s).split('/')[-1]
    s2 = strip(t)
    if len(s2) > 15 and any(0x0620 <= ord(c) <= 0x064A for c in s2[:10]):
        sh_texts[hid].append(s2)

indices = {}
for klen in [40, 30, 20, 15, 10]:
    idx = {}
    for hid, texts in sh_texts.items():
        for s2 in texts:
            if len(s2) > klen:
                idx[s2[-klen:]] = hid
                mid = len(s2) // 2
                if len(s2) > klen * 2:
                    idx[s2[mid-klen//2:mid+klen//2]] = hid
    indices[klen] = idx

sh_by_num = {}
for hid in sh_texts:
    m = re.match(r'(SB|SM|SD|JT|SN|IM)-HD(\d+)', hid)
    if m:
        sh_by_num[(m.group(1), int(m.group(2)))] = hid

book_to_prefix = {
    'صحيح البخاري': 'SB', 'صحيح مسلم': 'SM', 'سنن أبي داود': 'SD',
    'جامع الترمذي': 'JT', 'سنن النسائى الصغرى': 'SN', 'سنن ابن ماجه': 'IM',
}

def find_sem_hadith(matn, hadith_full, prefix, num):
    """Find matching SemanticHadith hadith ID using cascade."""
    for source in [matn, hadith_full]:
        if not source:
            continue
        s2 = strip(source)
        for klen in [40, 30, 20, 15, 10]:
            if len(s2) > klen:
                key = s2[-klen:]
                if key in indices[klen]:
                    return indices[klen][key]
                mid = len(s2) // 2
                if len(s2) > klen * 2:
                    key = s2[mid-klen//2:mid+klen//2]
                    if key in indices[klen]:
                        return indices[klen][key]
    # Number fallback
    if num > 0 and (prefix, num) in sh_by_num:
        return sh_by_num[(prefix, num)]
    return None

# ── Match and align chains ──────────────────────────────────────────────────
print("\nMatching hadiths and aligning chains...")

# Stats
total_hadiths = 0
matched_hadiths = 0
same_length = 0
diff_length = 0
total_positions = 0
aligned_positions = 0

# Name mapping: sanadset_stripped → {hn_id → count}
name_map = defaultdict(lambda: defaultdict(int))
# Per-position alignment samples
align_samples = []

per_book = defaultdict(lambda: {'total': 0, 'matched': 0, 'same_len': 0, 'positions': 0, 'aligned': 0})

with open('data/sanadset.csv') as f:
    reader = csv.DictReader(f)
    for row in reader:
        book = row.get('Book', '')
        if book not in book_to_prefix:
            continue
        prefix = book_to_prefix[book]
        num = int(row.get('Num_hadith', 0) or 0)
        matn = row.get('Matn', '')
        hadith_full = row.get('Hadith', '')
        sanad = row.get('Sanad', '')

        if not sanad or sanad == 'No SANAD':
            continue

        total_hadiths += 1
        per_book[book]['total'] += 1

        sem_hid = find_sem_hadith(matn, hadith_full, prefix, num)
        if not sem_hid or sem_hid not in sem_chains:
            continue

        matched_hadiths += 1
        per_book[book]['matched'] += 1

        try:
            san_names = ast.literal_eval(sanad)
        except:
            continue

        san_stripped = [strip_keep_spaces(n.strip()) for n in san_names]
        sem_chain = sem_chains[sem_hid]
        sem_names = [(hn, strip_keep_spaces(name)) for hn, name in sem_chain]

        if len(san_stripped) == len(sem_names):
            same_length += 1
            per_book[book]['same_len'] += 1
            # Direct positional alignment
            for i, (san_n, (hn, sem_n)) in enumerate(zip(san_stripped, sem_names)):
                total_positions += 1
                per_book[book]['positions'] += 1
                name_map[san_n][hn] += 1
                # Check if names are related (substring match)
                if san_n in sem_n or sem_n in san_n or san_n == sem_n:
                    aligned_positions += 1
                    per_book[book]['aligned'] += 1
                elif len(align_samples) < 20:
                    align_samples.append((book, num, i, san_n, sem_n, hn))
        else:
            diff_length += 1
            # Align from end (sahabi) — most reliable position
            # and from start (first transmitter)
            min_len = min(len(san_stripped), len(sem_names))

            # End alignment (last N narrators)
            end_align = min(min_len, 2)
            for j in range(1, end_align + 1):
                san_n = san_stripped[-j]
                hn, sem_n = sem_names[-j]
                name_map[san_n][hn] += 1
                total_positions += 1
                per_book[book]['positions'] += 1
                if san_n in sem_n or sem_n in san_n:
                    aligned_positions += 1
                    per_book[book]['aligned'] += 1

            # Start alignment (first N narrators)
            start_align = min(min_len - end_align, 2)
            for j in range(start_align):
                san_n = san_stripped[j]
                hn, sem_n = sem_names[j]
                name_map[san_n][hn] += 1
                total_positions += 1
                per_book[book]['positions'] += 1
                if san_n in sem_n or sem_n in san_n:
                    aligned_positions += 1
                    per_book[book]['aligned'] += 1

# ── Report ──────────────────────────────────────────────────────────────────
print(f"\n{'='*80}")
print("CHAIN ALIGNMENT RESULTS")
print(f"{'='*80}")
print(f"  Total hadiths (6 books): {total_hadiths}")
print(f"  Matched by text:         {matched_hadiths} ({100*matched_hadiths/total_hadiths:.1f}%)")
print(f"  Same chain length:       {same_length} ({100*same_length/matched_hadiths:.1f}% of matched)")
print(f"  Different chain length:  {diff_length}")
print(f"  Total positions compared: {total_positions}")
print(f"  Substring-aligned:        {aligned_positions} ({100*aligned_positions/total_positions:.1f}%)")

print(f"\n  Per-book:")
print(f"  {'Book':<30} {'Matched':>8} {'SameLen':>8} {'Positions':>10} {'Aligned':>8} {'%':>6}")
for book in sorted(per_book.keys()):
    b = per_book[book]
    pct = 100 * b['aligned'] / b['positions'] if b['positions'] else 0
    print(f"  {book:<30} {b['matched']:>8} {b['same_len']:>8} {b['positions']:>10} {b['aligned']:>8} {pct:>5.1f}%")

# ── Name mapping quality ────────────────────────────────────────────────────
print(f"\n{'='*80}")
print("NAME MAPPING QUALITY")
print(f"{'='*80}")

certain = 0
ambig = 0
total_names = len(name_map)
certain_mentions = 0
ambig_mentions = 0

for name, ids in name_map.items():
    total_count = sum(ids.values())
    if len(ids) == 1:
        certain += 1
        certain_mentions += total_count
    else:
        ambig += 1
        ambig_mentions += total_count

print(f"  Unique sanadset names mapped: {total_names}")
print(f"  Certain (maps to 1 ID):       {certain} ({100*certain/total_names:.1f}%)")
print(f"  Ambiguous (maps to >1 ID):    {ambig} ({100*ambig/total_names:.1f}%)")
print(f"  Certain mentions:             {certain_mentions}")
print(f"  Ambiguous mentions:           {ambig_mentions}")

# For ambiguous, check if there's a dominant mapping (>90% of occurrences)
dominant = 0
for name, ids in name_map.items():
    if len(ids) > 1:
        total_count = sum(ids.values())
        top_count = max(ids.values())
        if top_count / total_count > 0.9:
            dominant += 1

print(f"  Ambiguous with >90% dominant: {dominant} (could be resolved)")

# ── Alignment mismatches ────────────────────────────────────────────────────
print(f"\n{'='*80}")
print("POSITIONAL ALIGNMENT SAMPLES (where names DON'T substring-match)")
print(f"{'='*80}")
for book, num, pos, san, sem, hn in align_samples[:15]:
    info = narrator_info.get(hn, {})
    print(f"\n  {book} #{num} pos[{pos}]:")
    print(f"    Sanadset:  {san}")
    print(f"    Semantic:  {sem} [{hn}]")
    print(f"    Full name: {info.get('name', '-')[:60]}")

# ── Top certain mappings ────────────────────────────────────────────────────
print(f"\n{'='*80}")
print("TOP 20 CERTAIN MAPPINGS (most frequent)")
print(f"{'='*80}")
sorted_certain = [(n, ids) for n, ids in sorted(name_map.items(),
                   key=lambda x: -sum(x[1].values())) if len(ids) == 1]
for name, ids in sorted_certain[:20]:
    hn, cnt = list(ids.items())[0]
    info = narrator_info.get(hn, {})
    print(f"  {name} ({cnt}x) → {info.get('popularName', hn)} [{hn}] gen={info.get('generation','-')}")

# ── Top ambiguous ───────────────────────────────────────────────────────────
print(f"\n{'='*80}")
print("TOP 20 AMBIGUOUS MAPPINGS")
print(f"{'='*80}")
sorted_ambig = [(n, ids) for n, ids in sorted(name_map.items(),
                 key=lambda x: -sum(x[1].values())) if len(ids) > 1]
for name, ids in sorted_ambig[:20]:
    total_c = sum(ids.values())
    top_hn, top_cnt = max(ids.items(), key=lambda x: x[1])
    info = narrator_info.get(top_hn, {})
    pct = 100 * top_cnt / total_c
    print(f"  {name} ({total_c}x) → top: {info.get('popularName', top_hn)} [{top_hn}] {pct:.0f}% ({len(ids)} candidates)")
