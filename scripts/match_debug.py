#!/usr/bin/env python3
"""Debug: figure out why each book doesn't match 100% and fix it."""

import csv, sys, re, ast
from collections import defaultdict, Counter
from rdflib import Graph, Namespace

csv.field_size_limit(sys.maxsize)

def strip(t):
    t = re.sub(r'[\u0610-\u061A\u0640\u064B-\u065F\u0670\u06D6-\u06ED]', '', t)
    t = re.sub(r'[إأآٱ]', 'ا', t)
    t = t.replace('ة', 'ه').replace('ى', 'ي')
    # Remove ALL non-Arabic-letter chars (spaces, punctuation, quotes, etc.)
    t = re.sub(r'[^\u0620-\u064A]', '', t)
    return t

SH = Namespace('http://www.semantichadith.com/ontology/')
print("Loading SemanticHadith KG...")
g = Graph()
g.parse('/tmp/SemanticHadithKGV2.ttl', format='turtle')

# ── Step 1: Check what book names exist in sanadset vs what prefixes in SH ──

print("\n=== BOOK NAME DEBUG ===")
# Check Tirmidhi naming
sanadset_books = Counter()
with open('data/sanadset.csv') as f:
    for row in csv.DictReader(f):
        sanadset_books[row.get('Book', '')] += 1

for name, count in sanadset_books.most_common():
    if 'ترمذ' in name or 'تِرْمِذ' in name or 'جامع' in name:
        print(f"  Sanadset Tirmidhi: '{name}' ({count} hadiths)")

# SH prefixes
sh_books = Counter()
for s, p, o in g.triples((None, SH.hasNarratorChain, None)):
    hid = str(s).split('/')[-1]
    prefix = hid.split('-')[0]
    sh_books[prefix] += 1
print(f"  SH book prefixes: {dict(sh_books)}")

# ── Step 2: Build SH text index with MULTIPLE key strategies ────────────────

print("\n=== BUILDING MULTI-STRATEGY TEXT INDEX ===")

# Collect ALL Arabic fullHadithTexts
sh_texts = defaultdict(list)  # hadith_id → list of stripped Arabic texts
for s, p, o in g.triples((None, SH.fullHadithText, None)):
    t = str(o)
    hid = str(s).split('/')[-1]
    s2 = strip(t)
    # Must have Arabic letters
    if len(s2) > 20 and any(0x0620 <= ord(c) <= 0x064A for c in s2[:10]):
        sh_texts[hid].append(s2)

print(f"  {len(sh_texts)} hadiths with Arabic text")

# Check how many texts per hadith
multi = sum(1 for v in sh_texts.values() if len(v) > 1)
print(f"  {multi} hadiths with multiple Arabic texts")

# Build multiple indices
idx_end40 = {}     # last 40 chars
idx_end30 = {}     # last 30 chars
idx_end20 = {}     # last 20 chars
idx_mid40 = {}     # middle 40 chars

for hid, texts in sh_texts.items():
    for s2 in texts:
        if len(s2) > 50:
            idx_end40[s2[-40:]] = hid
            idx_end30[s2[-30:]] = hid
            idx_end20[s2[-20:]] = hid
            mid = len(s2) // 2
            idx_mid40[s2[mid-20:mid+20]] = hid

print(f"  Index sizes: end40={len(idx_end40)}, end30={len(idx_end30)}, end20={len(idx_end20)}, mid40={len(idx_mid40)}")

# ── Step 3: Match sanadset with cascading strategy ─────────────────────────

print("\n=== MATCHING WITH CASCADE ===")

book_map = {
    'صحيح البخاري': 'SB',
    'صحيح مسلم': 'SM',
    'سنن أبي داود': 'SD',
    'سنن الترمذي': 'JT',
    'جامع الترمذي': 'JT',          # alternate name
    'سنن النسائى الصغرى': 'SN',
    'السنن الصغرى للنسائي': 'SN',   # alternate
    'سنن ابن ماجه': 'IM',
}

per_book = defaultdict(lambda: {'total': 0, 'has_matn': 0, 'end40': 0, 'end30': 0, 'end20': 0, 'mid40': 0, 'any': 0})
unmatched_samples = defaultdict(list)  # book → list of (num, matn_preview)

with open('data/sanadset.csv') as f:
    for row in csv.DictReader(f):
        book = row.get('Book', '')
        if book not in book_map:
            continue

        b = per_book[book]
        b['total'] += 1

        matn = row.get('Matn', '')
        hadith_text = row.get('Hadith', '')  # full text including isnad
        num = row.get('Num_hadith', '')

        # Try matn first, then full hadith text
        for source_text in [matn, hadith_text]:
            if not source_text:
                continue
            s2 = strip(source_text)
            if len(s2) < 25:
                continue

            b['has_matn'] += 1
            matched = False

            if len(s2) > 50 and s2[-40:] in idx_end40:
                b['end40'] += 1
                matched = True
            elif len(s2) > 40 and s2[-30:] in idx_end30:
                b['end30'] += 1
                matched = True
            elif len(s2) > 30 and s2[-20:] in idx_end20:
                b['end20'] += 1
                matched = True
            elif len(s2) > 50:
                mid = len(s2) // 2
                if s2[mid-20:mid+20] in idx_mid40:
                    b['mid40'] += 1
                    matched = True

            if matched:
                b['any'] += 1
                break  # don't try hadith_text if matn matched
        else:
            # Neither matn nor hadith_text matched
            if len(unmatched_samples[book]) < 5:
                preview = strip(matn)[:80] if matn else strip(hadith_text)[:80]
                unmatched_samples[book].append((num, preview, len(strip(matn)) if matn else 0))

print(f"\n  {'Book':<30} {'Total':>6} {'HasText':>8} {'End40':>6} {'End30':>6} {'End20':>6} {'Mid40':>6} {'ANY':>6} {'%':>6}")
for book in sorted(per_book.keys()):
    b = per_book[book]
    pct = 100 * b['any'] / b['total'] if b['total'] else 0
    print(f"  {book:<30} {b['total']:>6} {b['has_matn']:>8} {b['end40']:>6} {b['end30']:>6} {b['end20']:>6} {b['mid40']:>6} {b['any']:>6} {pct:>5.1f}%")

total_all = sum(b['total'] for b in per_book.values())
matched_all = sum(b['any'] for b in per_book.values())
print(f"\n  TOTAL: {matched_all}/{total_all} ({100*matched_all/total_all:.1f}%)")

# ── Step 4: Debug unmatched ────────────────────────────────────────────────

print(f"\n=== UNMATCHED SAMPLES ===")
for book, samples in unmatched_samples.items():
    print(f"\n  {book}:")
    for num, preview, mlen in samples:
        print(f"    #{num} (matn_len={mlen}): {preview[:60]}...")

# ── Step 5: Check Muslim specifically ──────────────────────────────────────

print(f"\n=== MUSLIM DEEP DIVE ===")
# How many Muslim hadiths in SH?
muslim_sh = sum(1 for hid in sh_texts if hid.startswith('SM-'))
print(f"  SH Muslim hadiths with Arabic text: {muslim_sh}")
# Check a few SH Muslim texts
for hid in ['SM-HD0001', 'SM-HD0002', 'SM-HD0010']:
    if hid in sh_texts:
        for t in sh_texts[hid]:
            print(f"  {hid}: ...{t[-60:]}")

# Check a few sanadset Muslim matns
count = 0
with open('data/sanadset.csv') as f:
    for row in csv.DictReader(f):
        if 'مسلم' in row.get('Book', '') and row.get('Matn', ''):
            s2 = strip(row['Matn'])
            print(f"  Sanadset Muslim #{row['Num_hadith']}: ...{s2[-60:]}")
            count += 1
            if count >= 3:
                break
