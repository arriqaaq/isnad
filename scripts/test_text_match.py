#!/usr/bin/env python3
"""Match hadiths by Arabic text between SemanticHadith KG and sanadset.csv,
then use the matched chains to resolve narrator identities."""

import csv, sys, ast, re
from collections import defaultdict
from rdflib import Graph, Namespace

csv.field_size_limit(sys.maxsize)

def strip_diacritics(text):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', text).strip()
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه')
    t = t.replace('ى', 'ي')
    return ' '.join(t.split())

SH = Namespace("http://www.semantichadith.com/ontology/")

# ── Load SemanticHadith ─────────────────────────────────────────────────────

print("Loading SemanticHadith KG...")
g = Graph()
g.parse("/tmp/SemanticHadithKGV2.ttl", format="turtle")

# Extract narrator info
narrator_info = {}
for s in g.subjects(predicate=None, object=SH.HadithNarrator):
    hn = str(s).split('/')[-1]
    if not hn.startswith('HN'):
        continue
    rec = {}
    for p, o in g.predicate_objects(subject=s):
        prop = str(p).split('/')[-1]
        if prop in ('popularName', 'name', 'deathYear', 'generation', 'lineage',
                     'residence', 'teknonym', 'narratorID'):
            rec[prop] = str(o).replace('@ar', '').split('^^')[0]
    narrator_info[hn] = rec

print(f"  {len(narrator_info)} narrators")

# Extract hadith Arabic text
print("Extracting hadith texts...")
hadith_texts = {}  # hadith_id → arabic text
for s, p, o in g.triples((None, SH.hasHadithText, None)):
    hid = str(s).split('/')[-1]
    text_id = str(o).split('/')[-1]
    hadith_texts[hid] = text_id

# Get actual Arabic text content
text_content = {}
for s, p, o in g.triples((None, SH.hadithArabicText, None)):
    tid = str(s).split('/')[-1]
    text_content[tid] = str(o)

# Map hadith_id → arabic text
hadith_arabic = {}
for hid, tid in hadith_texts.items():
    if tid in text_content:
        hadith_arabic[hid] = text_content[tid]

print(f"  {len(hadith_arabic)} hadiths with Arabic text")

# Extract chains (same as before)
hadith_chain = {}
for s, p, o in g.triples((None, SH.hasNarratorChain, None)):
    hid = str(s).split('/')[-1]
    cid = str(o).split('/')[-1]
    hadith_chain[hid] = cid

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

# Reconstruct ordered chains
sem_chains = {}  # hadith_id → [(hn_id, name), ...]
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

print(f"  {len(sem_chains)} chains reconstructed")

# ── Build text matching index ───────────────────────────────────────────────

print("\nBuilding text matching index...")

# Use a 60-char substring from the middle of the hadith text as a key
# (skip the beginning which often has common isnad phrases)
sem_text_index = {}  # stripped_substring → hadith_id

for hid, text in hadith_arabic.items():
    stripped = strip_diacritics(text)
    chars = list(stripped)
    if len(chars) > 100:
        key = ''.join(chars[30:90])
    elif len(chars) > 40:
        key = ''.join(chars[15:])
    else:
        key = stripped
    if key and len(key) > 20:
        sem_text_index[key] = hid

print(f"  {len(sem_text_index)} hadith text keys indexed")

# ── Match against sanadset ──────────────────────────────────────────────────

print("\nMatching sanadset hadiths by text...")

matched = 0
not_matched = 0
total = 0
chain_pairs = []  # (sanadset_names, sem_chain) for matched hadiths

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        total += 1
        sanad = row.get('Sanad', '')
        if not sanad or sanad == 'No SANAD':
            continue

        # Use Matn (hadith content without isnad) for matching
        matn = row.get('Matn', '')
        if not matn:
            continue

        stripped_matn = strip_diacritics(matn)
        chars = list(stripped_matn)
        if len(chars) > 100:
            key = ''.join(chars[30:90])
        elif len(chars) > 40:
            key = ''.join(chars[15:])
        else:
            key = stripped_matn

        if not key or len(key) <= 20:
            continue

        sem_hid = sem_text_index.get(key)
        if sem_hid and sem_hid in sem_chains:
            matched += 1
            try:
                san_names = ast.literal_eval(sanad)
                san_stripped = [strip_diacritics(n.strip()) for n in san_names]
                chain_pairs.append((san_stripped, sem_chains[sem_hid], row.get('Book', '')))
            except:
                pass
        else:
            not_matched += 1

        if total % 100000 == 0:
            print(f"  ... {total} rows, {matched} matched")

print(f"\n  Total sanadset rows: {total}")
print(f"  Matched by text: {matched} ({100*matched/total:.1f}%)")
print(f"  Not matched: {not_matched}")

# ── Analyze chain alignment ────────────────────────────────────────────────

print(f"\n{'='*80}")
print("CHAIN ALIGNMENT ANALYSIS")
print(f"{'='*80}")

# For matched hadiths, compare sanadset chain with SemanticHadith chain
# This tells us: for each position in the sanadset chain, which SemanticHadith
# narrator (with ID) corresponds?

name_to_id_mappings = defaultdict(lambda: defaultdict(int))
# stripped_sanadset_name → {hn_id → count}

same_len = 0
diff_len = 0
for san_names, sem_chain, book in chain_pairs:
    sem_names = [(hn, strip_diacritics(name)) for hn, name in sem_chain]

    if len(san_names) == len(sem_names):
        same_len += 1
        # Direct positional alignment
        for san_n, (hn, sem_n) in zip(san_names, sem_names):
            name_to_id_mappings[san_n][hn] += 1
    else:
        diff_len += 1
        # Try to align by finding common names
        # Match from end (sahabi) and start (first transmitter)
        # Just do what we can
        if len(san_names) > 0 and len(sem_names) > 0:
            # Last narrator (sahabi) usually matches
            name_to_id_mappings[san_names[-1]][sem_names[-1][0]] += 1

print(f"  Same chain length: {same_len}")
print(f"  Different chain length: {diff_len}")
print(f"  Unique sanadset names with mappings: {len(name_to_id_mappings)}")

# How many are unambiguous (1 hn_id)?
unambig = 0
ambig = 0
for san_name, id_counts in name_to_id_mappings.items():
    if len(id_counts) == 1:
        unambig += 1
    else:
        ambig += 1

print(f"  Unambiguous mappings (1 ID): {unambig}")
print(f"  Ambiguous mappings (>1 ID): {ambig}")

# ── Show sample mappings ────────────────────────────────────────────────────

print(f"\n{'='*80}")
print("SAMPLE MAPPINGS (top 20 by frequency)")
print(f"{'='*80}")

# Sort by total count
sorted_mappings = sorted(name_to_id_mappings.items(),
                         key=lambda x: -sum(x[1].values()))

for san_name, id_counts in sorted_mappings[:20]:
    total_count = sum(id_counts.values())
    if len(id_counts) == 1:
        hn, cnt = list(id_counts.items())[0]
        info = narrator_info.get(hn, {})
        print(f"\n  {san_name} ({total_count}x) → {info.get('popularName', hn)} [{hn}]")
        print(f"    death={info.get('deathYear', '-')} gen={info.get('generation', '-')} {info.get('lineage', '')}")
    else:
        print(f"\n  {san_name} ({total_count}x) → AMBIGUOUS:")
        for hn, cnt in sorted(id_counts.items(), key=lambda x: -x[1])[:3]:
            info = narrator_info.get(hn, {})
            print(f"    {cnt}x → {info.get('popularName', hn)} [{hn}] death={info.get('deathYear', '-')}")
