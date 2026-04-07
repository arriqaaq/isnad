#!/usr/bin/env python3
"""Match hadiths by Arabic text between SemanticHadith and sanadset."""

import csv, sys, ast, re
from collections import defaultdict
from rdflib import Graph, Namespace, Literal

csv.field_size_limit(sys.maxsize)

def strip_diacritics(text):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', text).strip()
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه')
    t = t.replace('ى', 'ي')
    return ' '.join(t.split())

SH = Namespace("http://www.semantichadith.com/ontology/")

print("Loading SemanticHadith KG...")
g = Graph()
g.parse("/tmp/SemanticHadithKGV2.ttl", format="turtle")

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
                     'residence', 'teknonym', 'narratorID'):
            rec[prop] = str(o).replace('@ar', '').split('^^')[0]
    narrator_info[hn] = rec
print(f"  {len(narrator_info)} narrators")

# ── Extract Arabic fullHadithText ───────────────────────────────────────────
print("Extracting Arabic hadith texts...")
hadith_ar_text = {}  # hadith_id → arabic full text
for s, p, o in g.triples((None, SH.fullHadithText, None)):
    text = str(o)
    if '@ar' in text or any(ord(c) > 0x0600 and ord(c) < 0x06FF for c in text[:50]):
        hid = str(s).split('/')[-1]
        # Clean up RDF formatting
        clean = text.replace('@ar', '').replace('^^http://www.w3.org/2001/XMLSchema#string', '').strip()
        if any(ord(c) > 0x0600 and ord(c) < 0x06FF for c in clean[:20]):
            hadith_ar_text[hid] = clean

print(f"  {len(hadith_ar_text)} hadiths with Arabic text")

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

print(f"  {len(sem_chains)} chains")

# ── Build text index ────────────────────────────────────────────────────────
print("\nBuilding text index from SemanticHadith Arabic texts...")

# Use a 60-char normalized substring as key
sem_text_keys = {}  # key → hadith_id

for hid, text in hadith_ar_text.items():
    stripped = strip_diacritics(text)
    chars = stripped.replace(' ', '')
    if len(chars) > 120:
        key = chars[40:100]
    elif len(chars) > 60:
        key = chars[20:80]
    else:
        key = chars
    if key and len(key) > 20:
        sem_text_keys[key] = hid

print(f"  {len(sem_text_keys)} text keys indexed")

# ── Match sanadset by text ──────────────────────────────────────────────────
print(f"\nMatching sanadset.csv by text content...")

matched = 0
no_match = 0
total = 0
chain_pairs = []  # (san_names, sem_chain)

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        total += 1
        matn = row.get('Matn', '')
        sanad = row.get('Sanad', '')
        if not matn or not sanad or sanad == 'No SANAD':
            continue

        stripped = strip_diacritics(matn)
        chars = stripped.replace(' ', '')
        if len(chars) > 120:
            key = chars[40:100]
        elif len(chars) > 60:
            key = chars[20:80]
        else:
            key = chars

        if not key or len(key) <= 20:
            continue

        sem_hid = sem_text_keys.get(key)
        if sem_hid and sem_hid in sem_chains:
            matched += 1
            try:
                san_names = [strip_diacritics(n.strip()) for n in ast.literal_eval(sanad)]
                chain_pairs.append((san_names, sem_chains[sem_hid]))
            except:
                pass
        else:
            no_match += 1

        if total % 100000 == 0:
            print(f"  ... {total} rows, {matched} matched")

print(f"\n  Total rows: {total}")
print(f"  Matched by text: {matched} ({100*matched/total:.1f}%)")

# ── Analyze positional alignment ───────────────────────────────────────────

print(f"\n{'='*80}")
print("NARRATOR MAPPING FROM TEXT-MATCHED HADITHS")
print(f"{'='*80}")

name_to_ids = defaultdict(lambda: defaultdict(int))

same_len = 0
for san_names, sem_chain in chain_pairs:
    sem_list = [(hn, strip_diacritics(name)) for hn, name in sem_chain]
    if len(san_names) == len(sem_list):
        same_len += 1
        for sn, (hn, _) in zip(san_names, sem_list):
            name_to_ids[sn][hn] += 1

print(f"  Chain pairs: {len(chain_pairs)}")
print(f"  Same length: {same_len}")

# Count certain vs ambiguous
certain = 0
ambig = 0
for name, ids in name_to_ids.items():
    if len(ids) == 1:
        certain += 1
    else:
        ambig += 1

print(f"  Unique narrator names mapped: {len(name_to_ids)}")
print(f"  Certain (1 ID): {certain}")
print(f"  Ambiguous (>1 ID): {ambig}")

# Show top mappings
print(f"\n  TOP 20 CERTAIN MAPPINGS:")
sorted_certain = [(n, ids) for n, ids in sorted(name_to_ids.items(),
                   key=lambda x: -sum(x[1].values())) if len(ids) == 1]
for name, ids in sorted_certain[:20]:
    hn, cnt = list(ids.items())[0]
    info = narrator_info.get(hn, {})
    print(f"    {name} ({cnt}x) → {info.get('popularName', hn)} [{hn}] gen={info.get('generation','-')} d={info.get('deathYear','-')}")

print(f"\n  TOP 10 AMBIGUOUS MAPPINGS:")
sorted_ambig = [(n, ids) for n, ids in sorted(name_to_ids.items(),
                 key=lambda x: -sum(x[1].values())) if len(ids) > 1]
for name, ids in sorted_ambig[:10]:
    total_c = sum(ids.values())
    print(f"    {name} ({total_c}x) →")
    for hn, cnt in sorted(ids.items(), key=lambda x: -x[1])[:3]:
        info = narrator_info.get(hn, {})
        print(f"      {cnt}x → {info.get('popularName', hn)} [{hn}] d={info.get('deathYear','-')}")
