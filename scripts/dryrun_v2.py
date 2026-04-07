#!/usr/bin/env python3
"""Dry run v2: proper chain alignment using LCS-style matching for different-length chains."""

import csv, sys, re, ast
from collections import defaultdict, Counter
from rdflib import Graph, Namespace

csv.field_size_limit(sys.maxsize)

def strip(t):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', t)
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه').replace('ى', 'ي')
    return re.sub(r'[^\u0620-\u064A]', '', t)

def strip_spaces(t):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', t)
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه').replace('ى', 'ي')
    return ' '.join(t.split())

def names_match(san, sem):
    """Check if two names likely refer to the same person."""
    # Strip to comparable form
    a = strip(san)
    b = strip(sem)
    if not a or not b:
        return False
    # Exact
    if a == b:
        return True
    # One contains the other
    if a in b or b in a:
        return True
    # First word matches (e.g. "سفيان" vs "سفيان بن عيينه")
    wa = a[:min(len(a), 6)]
    wb = b[:min(len(b), 6)]
    if len(wa) >= 4 and wa == wb:
        return True
    return False

def align_chains(san_names, sem_chain):
    """Align sanadset names to SemanticHadith chain using greedy matching.
    Returns list of (san_name, hn_id_or_None) for each sanadset position."""

    sem_names = [(hn, strip_spaces(name)) for hn, name in sem_chain]
    result = [None] * len(san_names)
    used_sem = set()

    # Pass 1: exact or substring match, preserve order
    si = 0  # sem pointer
    for i, san in enumerate(san_names):
        san_s = strip_spaces(san)
        # Look forward in sem from current pointer
        for j in range(si, len(sem_names)):
            if j in used_sem:
                continue
            hn, sem_n = sem_names[j]
            if names_match(san_s, sem_n):
                result[i] = hn
                used_sem.add(j)
                si = j + 1
                break

    # Pass 2: for unmatched sanadset names, try matching any remaining sem (relaxed order)
    for i, san in enumerate(san_names):
        if result[i] is not None:
            continue
        san_s = strip_spaces(san)
        for j, (hn, sem_n) in enumerate(sem_names):
            if j in used_sem:
                continue
            if names_match(san_s, sem_n):
                result[i] = hn
                used_sem.add(j)
                break

    return result

SH = Namespace('http://www.semantichadith.com/ontology/')
print("Loading SemanticHadith KG...")
g = Graph()
g.parse('/tmp/SemanticHadithKGV2.ttl', format='turtle')

# Extract narrators
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

# Extract chains
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

sem_chains = {}
for hid, cid in hadith_chain.items():
    segs = chain_segs.get(cid, set())
    if not segs: continue
    all_followed = set(seg_follows.get(s) for s in segs if seg_follows.get(s))
    heads = [s for s in segs if s not in all_followed]
    if not heads: continue
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

# Build text index
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

def find_sem(matn, hadith_full, prefix, num):
    for source in [matn, hadith_full]:
        if not source: continue
        s2 = strip(source)
        for klen in [40, 30, 20, 15, 10]:
            if len(s2) > klen:
                key = s2[-klen:]
                if key in indices[klen]: return indices[klen][key]
                mid = len(s2) // 2
                if len(s2) > klen * 2:
                    key = s2[mid-klen//2:mid+klen//2]
                    if key in indices[klen]: return indices[klen][key]
    if num > 0 and (prefix, num) in sh_by_num:
        return sh_by_num[(prefix, num)]
    return None

# ── Match and align ─────────────────────────────────────────────────────────
print("\nMatching and aligning chains...")

name_map = defaultdict(lambda: defaultdict(int))
total_hadiths = 0
matched_hadiths = 0
total_positions = 0
resolved_positions = 0
unresolved_positions = 0
skip = {'ابي', 'ابيه', 'جده', 'عمه', 'امه', 'اخيه', 'ابيها'}

per_book = defaultdict(lambda: {'total': 0, 'matched': 0, 'positions': 0, 'resolved': 0})

with open('data/sanadset.csv') as f:
    reader = csv.DictReader(f)
    for row in reader:
        book = row.get('Book', '')
        if book not in book_to_prefix: continue
        prefix = book_to_prefix[book]
        num = int(row.get('Num_hadith', 0) or 0)
        sanad = row.get('Sanad', '')
        if not sanad or sanad == 'No SANAD': continue

        total_hadiths += 1
        per_book[book]['total'] += 1

        sem_hid = find_sem(row.get('Matn', ''), row.get('Hadith', ''), prefix, num)
        if not sem_hid or sem_hid not in sem_chains: continue

        matched_hadiths += 1
        per_book[book]['matched'] += 1

        try:
            san_names = [n.strip() for n in ast.literal_eval(sanad)]
        except:
            continue

        aligned = align_chains(san_names, sem_chains[sem_hid])

        for san_name, hn_id in zip(san_names, aligned):
            san_s = strip_spaces(san_name)
            if san_s in skip: continue

            total_positions += 1
            per_book[book]['positions'] += 1
            if hn_id:
                resolved_positions += 1
                per_book[book]['resolved'] += 1
                name_map[san_s][hn_id] += 1
            else:
                unresolved_positions += 1

# ── Report ──────────────────────────────────────────────────────────────────
print(f"\n{'='*80}")
print("RESULTS")
print(f"{'='*80}")
print(f"  Hadiths matched: {matched_hadiths}/{total_hadiths} ({100*matched_hadiths/total_hadiths:.1f}%)")
print(f"  Narrator positions: {total_positions}")
print(f"  Resolved (aligned to HN ID): {resolved_positions} ({100*resolved_positions/total_positions:.1f}%)")
print(f"  Unresolved: {unresolved_positions} ({100*unresolved_positions/total_positions:.1f}%)")

print(f"\n  Per-book:")
print(f"  {'Book':<30} {'Matched':>8} {'Positions':>10} {'Resolved':>10} {'%':>6}")
for book in sorted(per_book.keys()):
    b = per_book[book]
    pct = 100 * b['resolved'] / b['positions'] if b['positions'] else 0
    print(f"  {book:<30} {b['matched']:>8} {b['positions']:>10} {b['resolved']:>10} {pct:>5.1f}%")

# Name mapping quality
certain = sum(1 for ids in name_map.values() if len(ids) == 1)
ambig = sum(1 for ids in name_map.values() if len(ids) > 1)
certain_m = sum(sum(ids.values()) for ids in name_map.values() if len(ids) == 1)
ambig_m = sum(sum(ids.values()) for ids in name_map.values() if len(ids) > 1)

# For ambiguous, how many have >90% dominant?
dominant_90 = 0
dominant_95 = 0
for ids in name_map.values():
    if len(ids) > 1:
        total_c = sum(ids.values())
        top_c = max(ids.values())
        if top_c / total_c > 0.90: dominant_90 += 1
        if top_c / total_c > 0.95: dominant_95 += 1

print(f"\n  Unique names mapped: {len(name_map)}")
print(f"  Certain (1 ID):     {certain} ({100*certain/len(name_map):.1f}%) — {certain_m} mentions")
print(f"  Ambiguous (>1 ID):  {ambig} ({100*ambig/len(name_map):.1f}%) — {ambig_m} mentions")
print(f"  Ambiguous >90% dominant: {dominant_90}")
print(f"  Ambiguous >95% dominant: {dominant_95}")

# Top ambiguous
print(f"\n  TOP 10 AMBIGUOUS:")
sorted_ambig = sorted([(n, ids) for n, ids in name_map.items() if len(ids) > 1],
                       key=lambda x: -sum(x[1].values()))
for name, ids in sorted_ambig[:10]:
    total_c = sum(ids.values())
    top_hn, top_c = max(ids.items(), key=lambda x: x[1])
    info = narrator_info.get(top_hn, {})
    pct = 100 * top_c / total_c
    print(f"    {name} ({total_c}x) → {info.get('popularName', top_hn)} {pct:.0f}% | {len(ids)} candidates")
