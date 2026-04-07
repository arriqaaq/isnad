#!/usr/bin/env python3
"""Extract SemanticHadith KG V2 TTL into a JSON file for Rust ingestion.

Input:  /tmp/SemanticHadithKGV2.ttl (download via `make semantic-download`)
Output: data/semantic_hadith.json

Source: https://github.com/A-Kamran/SemanticHadith-V2
Paper:  Journal of Web Semantics, 2023
"""

import json
import os
import sys
import time
from collections import defaultdict

from rdflib import Graph, Namespace

TTL_PATH = "/tmp/SemanticHadithKGV2.ttl"
OUTPUT_PATH = "data/semantic_hadith.json"

SH = Namespace("http://www.semantichadith.com/ontology/")

BOOK_NAMES = {
    "SB": "صحيح البخاري",
    "SM": "صحيح مسلم",
    "SD": "سنن أبي داود",
    "JT": "جامع الترمذي",
    "SN": "سنن النسائى الصغرى",
    "IM": "سنن ابن ماجه",
}


def clean_literal(value):
    """Strip RDF type annotations and language tags from a literal value."""
    s = str(value)
    for suffix in ["@ar", "@en", "@ur"]:
        s = s.replace(suffix, "")
    if "^^" in s:
        s = s[: s.index("^^")]
    return s.strip()


def local_name(uri):
    """Extract local name from a full URI."""
    return str(uri).split("/")[-1]


def main():
    if not os.path.exists(TTL_PATH):
        print(f"ERROR: {TTL_PATH} not found. Run: make semantic-download")
        sys.exit(1)

    print(f"Loading {TTL_PATH} into rdflib...")
    t0 = time.time()
    g = Graph()
    g.parse(TTL_PATH, format="turtle")
    print(f"  Loaded {len(g)} triples in {time.time() - t0:.1f}s")

    # ── Extract narrators ───────────────────────────────────────────────────

    print("Extracting narrators...")
    narrators = {}

    NARRATOR_PROPS = {
        "popularName",
        "name",
        "teknonym",
        "generation",
        "lineage",
        "residence",
        "deathYear",
        "birthYear",
        "title",
        "office",
        "attribute",
        "narratorID",
    }

    for s in g.subjects(predicate=None, object=SH.HadithNarrator):
        hn = local_name(s)
        if not hn.startswith("HN"):
            continue
        rec = {}
        for p, o in g.predicate_objects(subject=s):
            prop = local_name(p)
            if prop in NARRATOR_PROPS:
                val = clean_literal(o)
                if val and val != "-":
                    rec[prop] = val
        if rec:
            narrators[hn] = rec

    print(f"  {len(narrators)} narrators extracted")

    # ── Extract hadith chains ───────────────────────────────────────────────

    print("Extracting hadith chains...")

    hadith_chain_id = {}
    for s, _, o in g.triples((None, SH.hasNarratorChain, None)):
        hadith_chain_id[local_name(s)] = local_name(o)

    chain_segs = defaultdict(set)
    for s, _, o in g.triples((None, SH.hasNarratorSegment, None)):
        chain_segs[local_name(s)].add(local_name(o))
    for s, _, o in g.triples((None, SH.hasRootNarratorSegment, None)):
        chain_segs[local_name(s)].add(local_name(o))

    seg_narrator = {}
    for s, _, o in g.triples((None, SH.refersToNarrator, None)):
        seg_narrator[local_name(s)] = local_name(o)

    seg_follows = {}
    for s, _, o in g.triples((None, SH.follows, None)):
        sid = local_name(s)
        if "ChainSeg" in sid:
            seg_follows[sid] = local_name(o)

    # Reconstruct ordered chains
    chains = {}  # hadith_id → [hn_id, ...]
    for hid, cid in hadith_chain_id.items():
        segs = chain_segs.get(cid, set())
        if not segs:
            continue
        # Find head: segment not followed by anyone
        all_followed = {seg_follows.get(s) for s in segs if seg_follows.get(s)}
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
                ordered.append(hn)
            current = seg_follows.get(current)
        if ordered:
            chains[hid] = ordered

    print(f"  {len(chains)} chains reconstructed")

    # ── Extract hadith metadata ─────────────────────────────────────────────

    print("Extracting hadith metadata...")

    # Reference numbers
    hadith_ref = {}
    for s, _, o in g.triples((None, SH.hadithReferenceNo, None)):
        hid = local_name(s)
        try:
            hadith_ref[hid] = int(clean_literal(o))
        except ValueError:
            pass

    # Hadith types
    hadith_types = {}
    for s, _, o in g.triples((None, SH.hasHadithType, None)):
        hadith_types[local_name(s)] = local_name(o)

    # Chapters
    hadith_chapters = {}
    for s, _, o in g.triples((None, SH.isPartOfChapter, None)):
        hadith_chapters[local_name(s)] = local_name(o)

    chapter_prefaces = {}
    for s, _, o in g.triples((None, SH.chapterPreface, None)):
        chapter_prefaces[local_name(s)] = clean_literal(o)

    # Topics
    hadith_topics = defaultdict(list)
    for s, _, o in g.triples((None, SH.discussesTopic, None)):
        hadith_topics[local_name(s)].append(local_name(o))

    # Entity mentions
    hadith_mentions = defaultdict(list)
    for s, _, o in g.triples((None, SH.containsMentionOf, None)):
        hadith_mentions[local_name(s)].append(local_name(o))

    # Quran verse mentions
    hadith_verses = defaultdict(list)
    for s, _, o in g.triples((None, SH.containsMentionOfVerse, None)):
        hadith_verses[local_name(s)].append(local_name(o))

    # Similar hadiths
    hadith_similar = defaultdict(list)
    for s, _, o in g.triples((None, SH.isSimilar, None)):
        hadith_similar[local_name(s)].append(local_name(o))

    hadith_strongly_similar = defaultdict(list)
    for s, _, o in g.triples((None, SH.isStronglySimilar, None)):
        hadith_strongly_similar[local_name(s)].append(local_name(o))

    # See also
    hadith_see_also = defaultdict(list)
    for s, _, o in g.triples((None, SH.seeAlso, None)):
        hid = local_name(s)
        ref = local_name(o)
        # Only include hadith cross-refs (HD pattern), not external
        if "-HD" in ref:
            hadith_see_also[hid].append(ref)

    print(
        f"  refs={len(hadith_ref)}, types={len(hadith_types)}, "
        f"chapters={len(hadith_chapters)}, topics={sum(len(v) for v in hadith_topics.values())}, "
        f"mentions={sum(len(v) for v in hadith_mentions.values())}, "
        f"verses={sum(len(v) for v in hadith_verses.values())}, "
        f"similar={sum(len(v) for v in hadith_similar.values())}"
    )

    # ── Extract texts ───────────────────────────────────────────────────────

    print("Extracting hadith texts...")

    texts = {}  # hadith_id → {"ar": ..., "en": ...}

    for s, _, o in g.triples((None, SH.fullHadithText, None)):
        hid = local_name(s)
        raw = str(o)
        cleaned = clean_literal(o)

        if not cleaned or len(cleaned) < 10:
            continue

        if hid not in texts:
            texts[hid] = {}

        if "@en" in raw:
            texts[hid]["en"] = cleaned
        elif any(0x0620 <= ord(c) <= 0x064A for c in cleaned[:20]):
            # Actual Arabic (not Urdu)
            if not any(0x0680 <= ord(c) <= 0x06FF for c in cleaned[:50]):
                texts[hid]["ar"] = cleaned
            # else: Urdu, skip

    print(
        f"  {len(texts)} hadiths with text, "
        f"{sum(1 for t in texts.values() if 'ar' in t)} with Arabic, "
        f"{sum(1 for t in texts.values() if 'en' in t)} with English"
    )

    # ── Assemble output ─────────────────────────────────────────────────────

    print("Assembling output...")

    hadiths_out = {}
    for hid in chains:
        prefix = hid.split("-")[0] if "-" in hid else ""
        if prefix not in BOOK_NAMES:
            continue

        rec = {
            "book": prefix,
            "bookName": BOOK_NAMES[prefix],
            "refNo": hadith_ref.get(hid),
            "chain": chains[hid],
        }

        if hid in hadith_types:
            rec["type"] = hadith_types[hid]
        if hid in hadith_chapters:
            ch = hadith_chapters[hid]
            rec["chapter"] = ch
            if ch in chapter_prefaces:
                rec["chapterPreface"] = chapter_prefaces[ch]
        if hid in hadith_topics:
            rec["topics"] = hadith_topics[hid]
        if hid in hadith_mentions:
            rec["mentions"] = hadith_mentions[hid]
        if hid in hadith_verses:
            rec["quranVerses"] = hadith_verses[hid]
        if hid in hadith_similar:
            rec["similar"] = hadith_similar[hid]
        if hid in hadith_strongly_similar:
            rec["stronglySimilar"] = hadith_strongly_similar[hid]
        if hid in hadith_see_also:
            rec["seeAlso"] = hadith_see_also[hid]

        # Text
        if hid in texts:
            rec["textAr"] = texts[hid].get("ar", "")
            rec["textEn"] = texts[hid].get("en", "")

        hadiths_out[hid] = rec

    output = {
        "narrators": narrators,
        "hadiths": hadiths_out,
        "bookNames": BOOK_NAMES,
    }

    # ── Write ───────────────────────────────────────────────────────────────

    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
    print(f"Writing {OUTPUT_PATH}...")
    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=None)

    size_mb = os.path.getsize(OUTPUT_PATH) / 1_000_000
    print(f"  {size_mb:.1f} MB written")

    # Stats
    books = defaultdict(int)
    for h in hadiths_out.values():
        books[h["book"]] += 1

    print(f"\nSummary:")
    print(f"  Narrators: {len(narrators)}")
    print(f"  Hadiths:   {len(hadiths_out)}")
    for prefix in sorted(books):
        print(f"    {BOOK_NAMES[prefix]}: {books[prefix]}")
    print(f"  With Arabic text: {sum(1 for h in hadiths_out.values() if h.get('textAr'))}")
    print(f"  With English text: {sum(1 for h in hadiths_out.values() if h.get('textEn'))}")


if __name__ == "__main__":
    main()
