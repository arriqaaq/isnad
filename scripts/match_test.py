#!/usr/bin/env python3
"""Quick test: strip diacritics from sanadset names, match against ar_sanad_narrators.
No lemmatization, no kunya normalization — just raw diacritic stripping."""

import csv, sys, ast, re
from collections import Counter, defaultdict

csv.field_size_limit(sys.maxsize)

def strip_diacritics(text):
    """Remove Arabic diacritics only. No letter normalization."""
    return re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', text).strip()

def normalize_light(text):
    """Strip diacritics + normalize alef/taa/yaa variants + collapse whitespace."""
    t = strip_diacritics(text)
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه')
    t = t.replace('ى', 'ي')
    return ' '.join(t.split())

def normalize_with_kunya(text):
    """Light normalize + kunya case normalization (ابي/ابى/ابا → ابو)."""
    t = normalize_light(text)
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

# ── Load ar_sanad_narrators ─────────────────────────────────────────────────

print("Loading ar_sanad_narrators.csv...")
narrators = {}
index_light = defaultdict(set)      # normalize_light form → IDs
index_kunya = defaultdict(set)      # normalize_with_kunya form → IDs

with open("data/ar_sanad_narrators.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        nid = int(row['id'])
        narrators[nid] = row['name']

        # Index primary name
        for norm_fn, idx in [(normalize_light, index_light), (normalize_with_kunya, index_kunya)]:
            n = norm_fn(row['name'])
            if n: idx[n].add(nid)

            # Namings
            try:
                for alias in ast.literal_eval(row.get('namings', '[]')):
                    na = norm_fn(alias.strip())
                    if na: idx[na].add(nid)
            except: pass

            # Kunia
            kunia = row.get('kunia', '').strip()
            if kunia and kunia != '-':
                nk = norm_fn(kunia)
                if nk: idx[nk].add(nid)

print(f"  {len(narrators)} narrators")
print(f"  {len(index_light)} unique light-normalized forms")
print(f"  {len(index_kunya)} unique kunya-normalized forms")

# ── Process sanadset ────────────────────────────────────────────────────────

print("\nProcessing ALL of sanadset.csv...")

skip_words = {'ابي', 'ابيه', 'جده', 'عمه', 'امه', 'اخيه', 'ابيها'}

stats = {
    'light': {'resolved': 0, 'ambiguous': 0, 'unresolved': 0},
    'kunya': {'resolved': 0, 'ambiguous': 0, 'unresolved': 0},
}
total = 0
skipped = 0
no_sanad = 0
rows = 0

unresolved_light = Counter()
unresolved_kunya = Counter()
ambiguous_kunya = Counter()

with open("data/sanadset.csv", encoding="utf-8") as f:
    reader = csv.DictReader(f)
    for row in reader:
        rows += 1
        sanad = row.get('Sanad', '')
        if not sanad or sanad == 'No SANAD':
            no_sanad += 1
            continue
        try:
            names = ast.literal_eval(sanad)
        except:
            continue

        for name in names:
            name = name.strip()
            if not name:
                continue

            nl = normalize_light(name)
            nk = normalize_with_kunya(name)

            if nk in skip_words or nl in skip_words:
                skipped += 1
                continue

            total += 1

            # Light (diacritics only)
            ids = index_light.get(nl, set())
            if len(ids) == 1:
                stats['light']['resolved'] += 1
            elif len(ids) > 1:
                stats['light']['ambiguous'] += 1
            else:
                stats['light']['unresolved'] += 1
                unresolved_light[nl] += 1

            # Kunya-normalized
            ids = index_kunya.get(nk, set())
            if len(ids) == 1:
                stats['kunya']['resolved'] += 1
            elif len(ids) > 1:
                stats['kunya']['ambiguous'] += 1
                ambiguous_kunya[nk] += 1
            else:
                stats['kunya']['unresolved'] += 1
                unresolved_kunya[nk] += 1

        if rows % 100000 == 0:
            print(f"  ... {rows} rows")

# ── Report ──────────────────────────────────────────────────────────────────

print(f"\n{'='*80}")
print(f"RESULTS — {rows} hadith rows, {no_sanad} without sanad, {total} narrator mentions, {skipped} relative refs skipped")
print(f"{'='*80}")

useful = total
for method in ['light', 'kunya']:
    s = stats[method]
    print(f"\n  {method.upper()} NORMALIZATION:")
    print(f"    Resolved (unique):  {s['resolved']:>10} ({100*s['resolved']/useful:.1f}%)")
    print(f"    Ambiguous (multi):  {s['ambiguous']:>10} ({100*s['ambiguous']/useful:.1f}%)")
    print(f"    Unresolved (none):  {s['unresolved']:>10} ({100*s['unresolved']/useful:.1f}%)")
    print(f"    Match rate:         {100*(s['resolved']+s['ambiguous'])/useful:.1f}%")

print(f"\n{'='*80}")
print("TOP 30 UNRESOLVED (kunya-normalized) by frequency")
print(f"{'='*80}")
for norm, count in unresolved_kunya.most_common(30):
    print(f"  [{count:>6}x] {norm}")

print(f"\n{'='*80}")
print("TOP 30 AMBIGUOUS (kunya-normalized) by frequency")
print(f"{'='*80}")
for norm, count in ambiguous_kunya.most_common(30):
    ids = list(index_kunya.get(norm, set()))
    names_sample = [narrators.get(i, '?')[:40] for i in ids[:3]]
    print(f"  [{count:>6}x] {norm}  → {len(ids)} candidates: {names_sample}")

print(f"\n  Unique unresolved forms (kunya): {len(unresolved_kunya)}")
print(f"  Unique ambiguous forms (kunya):  {len(ambiguous_kunya)}")
