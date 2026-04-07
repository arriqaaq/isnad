#!/usr/bin/env python3
"""Can we match SemanticHadith narrators to sanadset narrators for bio enrichment?"""

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

# ── Load SemanticHadith narrators ───────────────────────────────────────────

print("Loading SemanticHadith KG...")
g = Graph()
g.parse("/tmp/SemanticHadithKGV2.ttl", format="turtle")
print(f"  {len(g)} triples")

sem_narrators = {}  # hn_id → {popularName, name, deathYear, deathPlace, ...}

# Get all narrator metadata
for s in g.subjects(predicate=None, object=SH.HadithNarrator):
    hn = str(s).split('/')[-1]
    if not hn.startswith('HN'):
        continue
    rec = {'id': hn}
    for p, o in g.predicate_objects(subject=s):
        prop = str(p).split('/')[-1]
        if prop in ('popularName', 'name', 'deathYear', 'deathPlace', 'generation',
                     'lineage', 'residence', 'age', 'teknonym', 'title', 'office',
                     'narratorID', 'narratorURL', 'firstChar'):
            val = str(o).replace('@ar', '').replace('^^http://www.w3.org/2001/XMLSchema#string', '')
            rec[prop] = val
        # Wikidata link
        if 'wikidata' in str(o):
            rec['wikidata'] = str(o)
        if 'dbpedia' in str(o):
            rec.setdefault('dbpedia', []).append(str(o))
    sem_narrators[hn] = rec

print(f"  {len(sem_narrators)} narrators extracted")

# Show what fields are available
sample = list(sem_narrators.values())[0]
print(f"  Sample fields: {list(sample.keys())}")

# Count field coverage
fields = ['popularName', 'name', 'deathYear', 'deathPlace', 'generation',
          'lineage', 'residence', 'age', 'teknonym', 'title', 'office', 'wikidata']
print(f"\n  Field coverage:")
for field in fields:
    count = sum(1 for r in sem_narrators.values() if field in r)
    print(f"    {field:<15} {count:>5} ({100*count/len(sem_narrators):.0f}%)")

# ── Build SemanticHadith name index ─────────────────────────────────────────

sem_index = defaultdict(set)  # normalized name → set of HN ids
for hn, rec in sem_narrators.items():
    for field in ('popularName', 'name'):
        if field in rec:
            norm = strip_diacritics(rec[field])
            if norm:
                sem_index[norm].add(hn)

print(f"\n  {len(sem_index)} unique normalized forms in SemanticHadith index")

# ── Collect unique narrators from sanadset ──────────────────────────────────

print("\nCollecting unique narrators from sanadset.csv...")

sanad_narrators = defaultdict(int)  # normalized name → occurrence count

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        sanad = row.get('Sanad', '')
        if not sanad or sanad == 'No SANAD':
            continue
        try:
            names = ast.literal_eval(sanad)
        except:
            continue
        for name in names:
            norm = strip_diacritics(name.strip())
            if norm:
                sanad_narrators[norm] += 1

print(f"  {len(sanad_narrators)} unique normalized narrator forms in sanadset")
print(f"  {sum(sanad_narrators.values())} total mentions")

# ── Match ───────────────────────────────────────────────────────────────────

print("\nMatching sanadset narrators → SemanticHadith bio...")

matched_unique = 0
matched_mentions = 0
ambiguous_unique = 0
ambiguous_mentions = 0
unmatched_unique = 0
unmatched_mentions = 0

matched_hn_ids = set()

for norm, count in sanad_narrators.items():
    hits = sem_index.get(norm, set())
    if len(hits) == 1:
        matched_unique += 1
        matched_mentions += count
        matched_hn_ids.update(hits)
    elif len(hits) > 1:
        ambiguous_unique += 1
        ambiguous_mentions += count
    else:
        unmatched_unique += 1
        unmatched_mentions += count

total_u = len(sanad_narrators)
total_m = sum(sanad_narrators.values())

print(f"\n  {'Category':<20} {'Unique forms':>12} {'Mentions':>12}")
print(f"  {'-'*44}")
print(f"  {'Matched (1 hit)':<20} {matched_unique:>12} ({100*matched_unique/total_u:.1f}%) {matched_mentions:>12} ({100*matched_mentions/total_m:.1f}%)")
print(f"  {'Ambiguous (>1)':<20} {ambiguous_unique:>12} ({100*ambiguous_unique/total_u:.1f}%) {ambiguous_mentions:>12} ({100*ambiguous_mentions/total_m:.1f}%)")
print(f"  {'No match':<20} {unmatched_unique:>12} ({100*unmatched_unique/total_u:.1f}%) {unmatched_mentions:>12} ({100*unmatched_mentions/total_m:.1f}%)")
print(f"\n  Unique SemanticHadith narrators matched: {len(matched_hn_ids)} / {len(sem_narrators)}")

# ── Show what bio data we'd gain ────────────────────────────────────────────

print(f"\n{'='*80}")
print("WHAT BIO DATA WE'D GAIN for matched narrators")
print(f"{'='*80}")

for field in fields:
    count = sum(1 for hn in matched_hn_ids if field in sem_narrators.get(hn, {}))
    print(f"  {field:<15} {count:>5} narrators would get this field")

# ── Sample matched narrators ────────────────────────────────────────────────

print(f"\n{'='*80}")
print("SAMPLE MATCHED NARRATORS (top 10 by mention count)")
print(f"{'='*80}")

# Get top mentioned narrators that matched
matched_list = []
for norm, count in sorted(sanad_narrators.items(), key=lambda x: -x[1]):
    hits = sem_index.get(norm, set())
    if len(hits) == 1:
        hn = list(hits)[0]
        rec = sem_narrators[hn]
        matched_list.append((norm, count, rec))
        if len(matched_list) >= 10:
            break

for norm, count, rec in matched_list:
    print(f"\n  {norm} ({count}x mentions)")
    print(f"    popularName: {rec.get('popularName', '-')}")
    print(f"    full name:   {rec.get('name', '-')[:80]}")
    print(f"    death: {rec.get('deathYear', '-')} | residence: {rec.get('residence', '-')} | gen: {rec.get('generation', '-')}")
    print(f"    wikidata: {rec.get('wikidata', '-')}")
