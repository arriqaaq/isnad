#!/usr/bin/env python3
"""Measure: for ambiguous names, how many can chain-context resolve with CERTAINTY
(exactly 1 candidate passes both narrated_from AND narrated_to checks)?"""

import csv, sys, ast, re
from collections import Counter, defaultdict

csv.field_size_limit(sys.maxsize)

def normalize(text):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', text).strip()
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه')
    t = t.replace('ى', 'ي')
    t = ' '.join(t.split())
    words = t.split()
    result = []
    i = 0
    while i < len(words):
        w = words[i]
        if i + 1 < len(words) and w in ('ابي', 'ابى', 'ابا'):
            result.append('ابو')
        elif i + 1 < len(words) and w in ('امي', 'اما'):
            result.append('ام')
        else:
            result.append(w)
        i += 1
    return ' '.join(result)

skip_words = {'ابي', 'ابيه', 'جده', 'عمه', 'امه', 'اخيه', 'ابيها'}

# ── Load narrators ──────────────────────────────────────────────────────────

print("Loading ar_sanad_narrators.csv...")
narrators = {}
index = defaultdict(set)

with open("data/ar_sanad_narrators.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        nid = int(row['id'])
        nf = []
        nt = []
        try: nf = [int(x) for x in ast.literal_eval(row.get('narrated_from', '[]'))]
        except: pass
        try: nt = [int(x) for x in ast.literal_eval(row.get('narrated_to', '[]'))]
        except: pass
        narrators[nid] = {'name': row['name'], 'nf': set(nf), 'nt': set(nt)}

        for form in [row['name']] + ast.literal_eval(row.get('namings', '[]')):
            n = normalize(form.strip())
            if n: index[n].add(nid)
        kunia = row.get('kunia', '').strip()
        if kunia and kunia != '-':
            nk = normalize(kunia)
            if nk: index[nk].add(nid)

print(f"  {len(narrators)} narrators loaded")

# ── Process chains ──────────────────────────────────────────────────────────

print("\nProcessing sanadset.csv chains...")

# Counters
total_mentions = 0
certain_nocontext = 0       # unique match, no context needed
certain_withcontext = 0     # ambiguous, but context gives exactly 1
still_ambiguous = 0         # context didn't help or gave multiple
unresolved = 0              # no match at all
skipped_count = 0

# For detail
ambig_after_context = Counter()  # norm → count still ambiguous
certain_context_examples = []

rows = 0
with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        rows += 1
        sanad = row.get('Sanad', '')
        if not sanad or sanad == 'No SANAD':
            continue
        try:
            names = ast.literal_eval(sanad)
        except:
            continue

        # Normalize all names in this chain
        chain = []
        for name in names:
            name = name.strip()
            if not name:
                continue
            n = normalize(name)
            if n in skip_words:
                skipped_count += 1
                chain.append((n, 'skip', set()))
                continue
            ids = index.get(n, set())
            chain.append((n, 'pending', ids))

        # Pass 1: resolve unique matches
        resolved = [None] * len(chain)
        for i, (n, status, ids) in enumerate(chain):
            if status == 'skip':
                continue
            if len(ids) == 1:
                resolved[i] = next(iter(ids))

        # Pass 2 & 3: context disambiguation
        for _ in range(2):
            for i, (n, status, ids) in enumerate(chain):
                if status == 'skip' or resolved[i] is not None:
                    continue
                if len(ids) <= 1:
                    continue

                prev_id = resolved[i-1] if i > 0 else None
                next_id = resolved[i+1] if i+1 < len(chain) else None

                if prev_id is None and next_id is None:
                    continue

                # Score each candidate
                scored = []
                for cid in ids:
                    rec = narrators.get(cid)
                    if not rec:
                        continue
                    score = 0
                    # prev heard from this → this.narrated_to should contain prev
                    if prev_id is not None and prev_id in rec['nt']:
                        score += 1
                    # this heard from next → this.narrated_from should contain next
                    if next_id is not None and next_id in rec['nf']:
                        score += 1
                    if score > 0:
                        scored.append((cid, score))

                if len(scored) == 1:
                    resolved[i] = scored[0][0]
                elif len(scored) > 1:
                    # Multiple passed — pick only max score
                    max_s = max(s for _, s in scored)
                    best = [c for c, s in scored if s == max_s]
                    if len(best) == 1:
                        resolved[i] = best[0]

        # Count results
        for i, (n, status, ids) in enumerate(chain):
            if status == 'skip':
                continue
            total_mentions += 1
            if len(ids) == 0:
                unresolved += 1
            elif len(ids) == 1:
                certain_nocontext += 1
            elif resolved[i] is not None:
                certain_withcontext += 1
            else:
                still_ambiguous += 1
                ambig_after_context[n] += 1

        if rows % 100000 == 0:
            print(f"  ... {rows} rows")

# ── Report ──────────────────────────────────────────────────────────────────

print(f"\n{'='*80}")
print("CERTAINTY REPORT")
print(f"{'='*80}")
print(f"  Total mentions:           {total_mentions:>10}")
print(f"  Skipped (relatives):      {skipped_count:>10}")
print()
print(f"  CERTAIN (unique match):   {certain_nocontext:>10} ({100*certain_nocontext/total_mentions:.1f}%)")
print(f"  CERTAIN (context=1):      {certain_withcontext:>10} ({100*certain_withcontext/total_mentions:.1f}%)")
print(f"  ─────────────────────────────────────────")
print(f"  TOTAL CERTAIN:            {certain_nocontext+certain_withcontext:>10} ({100*(certain_nocontext+certain_withcontext)/total_mentions:.1f}%)")
print()
print(f"  STILL AMBIGUOUS:          {still_ambiguous:>10} ({100*still_ambiguous/total_mentions:.1f}%)")
print(f"  UNRESOLVED:               {unresolved:>10} ({100*unresolved/total_mentions:.1f}%)")
print(f"  ─────────────────────────────────────────")
print(f"  UNCERTAIN TOTAL:          {still_ambiguous+unresolved:>10} ({100*(still_ambiguous+unresolved)/total_mentions:.1f}%)")

print(f"\n{'='*80}")
print("TOP 30 STILL AMBIGUOUS after chain context (by mention count)")
print(f"{'='*80}")
for n, count in ambig_after_context.most_common(30):
    ids = index.get(n, set())
    print(f"  [{count:>6}x] {n}  ({len(ids)} candidates)")
