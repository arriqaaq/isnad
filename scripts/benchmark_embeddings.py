#!/usr/bin/env python3
"""
Benchmark Arabic embedding models for Islamic text semantic search.

Compares 5 models on a curated corpus of Quran ayahs + hadith texts,
measuring retrieval quality with ground-truth queries.

Usage:
    pip install sentence-transformers numpy torch
    python scripts/benchmark_embeddings.py                # all models, summary only
    python scripts/benchmark_embeddings.py --verbose      # per-query breakdown
    python scripts/benchmark_embeddings.py --models BGE-M3,MultilingualE5Small

How it works
============

When you search for Islamic texts (Quran ayahs, hadiths) by meaning — e.g.
searching "fasting" and finding relevant verses — you need a way to convert text
into numbers that capture its *meaning*. These numbers are called "embeddings".
Different AI models produce different quality embeddings, especially for Arabic.
This script figures out which model is best for this use case.

The 3 key pieces:

1. Models (MODEL_CONFIGS)
   Five embedding models are tested. Each converts text into a vector (a list of
   numbers). Key differences between models:
   - dim: vector size (384 to 1024). Larger = more expressive but slower.
   - query_prefix / passage_prefix: some models (like E5) need you to prepend
     "query: " or "passage: " so the model knows how the text is being used
     (searching vs. being searched).

2. Corpus (CORPUS)
   A hand-picked set of ~26 passages: Quran ayahs and hadiths in both Arabic and
   English. Some are "positive controls" (related to prayer, fasting, Eid) and
   some are "negative controls" (about usury, tawhid, equality) — texts that
   should NOT match prayer/fasting queries.

3. Queries with ground truth (QUERIES)
   15 test queries like "عيد" (Eid), "fasting", "صلاة" (prayer). Each has an
   `expected` list — the corpus IDs that a good model should rank highly. This
   is how we objectively score the models.

Benchmark flow (run_benchmark):
   For each model:
   a) Encode all passages into vectors (with passage prefix).
   b) Encode all queries into vectors (with query prefix).
   c) Compute cosine similarity (dot product of normalized vectors). Higher
      score = more semantically similar.
   d) Rank passages by score for each query.
   e) Measure quality using MRR, Recall@K, Spread, and Gap (see below).

Metrics:
   - MRR (Mean Reciprocal Rank): if the first correct result is at position N,
     score = 1/N. Averaged across all queries. Higher = correct results appear
     earlier.
   - Recall@K: of all expected results, what fraction appeared in the top K?
     E.g. if 2 of 3 expected hits are in the top 5, Recall@5 = 67%.
   - Spread: standard deviation of scores. Low spread means the model scores
     everything similarly — it can't tell relevant from irrelevant.
   - Gap: score difference between rank #1 and rank #5. Higher = the model is
     more confident about what's relevant.

Output:
   - Summary table: all models side by side, sorted by MRR.
   - Category breakdown: performance split by query type (short Arabic words,
     English phrases, conceptual queries). Reveals if a model is good at English
     but bad at single Arabic words.
"""

import argparse
import gc
import sys
import time
from dataclasses import dataclass, field

import numpy as np

# ──────────────────────────────────────────────────────────────────────
# Model configurations
# ──────────────────────────────────────────────────────────────────────

MODEL_CONFIGS = {
    "MultilingualE5Small": {
        "model_id": "intfloat/multilingual-e5-small",
        "dim": 384,
        "query_prefix": "query: ",
        "passage_prefix": "passage: ",
    },
    "MultilingualE5Large": {
        "model_id": "intfloat/multilingual-e5-large",
        "dim": 1024,
        "query_prefix": "query: ",
        "passage_prefix": "passage: ",
    },
    "BGE-M3": {
        "model_id": "BAAI/bge-m3",
        "dim": 1024,
        "query_prefix": "",
        "passage_prefix": "",
    },
    "Arabic-Triplet-Matryoshka-V2": {
        "model_id": "Omartificial-Intelligence-Space/Arabic-Triplet-Matryoshka-V2",
        "dim": 768,
        "query_prefix": "",
        "passage_prefix": "",
    },
    "Swan-Large": {
        "model_id": "Omartificial-Intelligence-Space/Swan-Large",
        "dim": 1024,
        "query_prefix": "query: ",
        "passage_prefix": "passage: ",
    },
}

# ──────────────────────────────────────────────────────────────────────
# Test corpus: Quran ayahs + hadith texts (Arabic and English)
#
# Think of this as a small "library" or "database" of texts. Each entry
# has a unique ID that acts as its label, e.g.:
#
#   q_2_183_ar  → Quran 2:183 in Arabic (about fasting)
#   q_2_183_en  → Quran 2:183 in English (same verse, translated)
#   h_fasting_ar → Hadith from Bukhari 38 in Arabic (about fasting)
#
# Some entries are "negative controls" — texts on unrelated topics
# (usury, tawhid, equality) that should NOT match fasting/prayer
# queries. A good model ranks these low for those queries.
# ──────────────────────────────────────────────────────────────────────

CORPUS = [
    # === Quran: Eid / Festival ===
    {
        "id": "q_5_114_ar",
        "text": "قَالَ عِيسَى ٱبْنُ مَرْيَمَ ٱللَّهُمَّ رَبَّنَآ أَنزِلْ عَلَيْنَا مَآئِدَةً مِّنَ ٱلسَّمَآءِ تَكُونُ لَنَا عِيدًا لِّأَوَّلِنَا وَءَاخِرِنَا وَءَايَةً مِّنكَ وَٱرْزُقْنَا وَأَنتَ خَيْرُ ٱلرَّٰزِقِينَ",
        "lang": "ar",
        "ref": "Quran 5:114",
    },
    {
        "id": "q_5_114_en",
        "text": 'Said Jesus, the son of Mary, "O Allah, our Lord, send down to us a table spread with food from the heaven to be for us a festival for the first of us and the last of us and a sign from You. And provide for us, and You are the best of providers."',
        "lang": "en",
        "ref": "Quran 5:114",
    },
    # === Quran: Fasting ===
    {
        "id": "q_2_183_ar",
        "text": "يَٰٓأَيُّهَا ٱلَّذِينَ ءَامَنُوا۟ كُتِبَ عَلَيْكُمُ ٱلصِّيَامُ كَمَا كُتِبَ عَلَى ٱلَّذِينَ مِن قَبْلِكُمْ لَعَلَّكُمْ تَتَّقُونَ",
        "lang": "ar",
        "ref": "Quran 2:183",
    },
    {
        "id": "q_2_183_en",
        "text": "O you who have believed, decreed upon you is fasting as it was decreed upon those before you that you may become righteous -",
        "lang": "en",
        "ref": "Quran 2:183",
    },
    # === Quran: Ramadan ===
    {
        "id": "q_2_185_ar",
        "text": "شَهْرُ رَمَضَانَ ٱلَّذِىٓ أُنزِلَ فِيهِ ٱلْقُرْءَانُ هُدًى لِّلنَّاسِ وَبَيِّنَٰتٍ مِّنَ ٱلْهُدَىٰ وَٱلْفُرْقَانِ فَمَن شَهِدَ مِنكُمُ ٱلشَّهْرَ فَلْيَصُمْهُ وَمَن كَانَ مَرِيضًا أَوْ عَلَىٰ سَفَرٍ فَعِدَّةٌ مِّنْ أَيَّامٍ أُخَرَ يُرِيدُ ٱللَّهُ بِكُمُ ٱلْيُسْرَ وَلَا يُرِيدُ بِكُمُ ٱلْعُسْرَ وَلِتُكْمِلُوا۟ ٱلْعِدَّةَ وَلِتُكَبِّرُوا۟ ٱللَّهَ عَلَىٰ مَا هَدَىٰكُمْ وَلَعَلَّكُمْ تَشْكُرُونَ",
        "lang": "ar",
        "ref": "Quran 2:185",
    },
    {
        "id": "q_2_185_en",
        "text": "The month of Ramadhan is that in which was revealed the Quran, a guidance for the people and clear proofs of guidance and criterion. So whoever sights the new moon of the month, let him fast it; and whoever is ill or on a journey - then an equal number of other days. Allah intends for you ease and does not intend for you hardship and wants for you to complete the period and to glorify Allah for that to which He has guided you; and perhaps you will be grateful.",
        "lang": "en",
        "ref": "Quran 2:185",
    },
    # === Quran: Prayer + Zakah ===
    {
        "id": "q_2_43_ar",
        "text": "وَأَقِيمُوا۟ ٱلصَّلَوٰةَ وَءَاتُوا۟ ٱلزَّكَوٰةَ وَٱرْكَعُوا۟ مَعَ ٱلرَّٰكِعِينَ",
        "lang": "ar",
        "ref": "Quran 2:43",
    },
    {
        "id": "q_2_43_en",
        "text": "And establish prayer and give zakah and bow with those who bow in worship and obedience.",
        "lang": "en",
        "ref": "Quran 2:43",
    },
    # === Quran: Guard prayers ===
    {
        "id": "q_2_238_ar",
        "text": "حَٰفِظُوا۟ عَلَى ٱلصَّلَوَٰتِ وَٱلصَّلَوٰةِ ٱلْوُسْطَىٰ وَقُومُوا۟ لِلَّهِ قَٰنِتِينَ",
        "lang": "ar",
        "ref": "Quran 2:238",
    },
    {
        "id": "q_2_238_en",
        "text": "Maintain with care the obligatory prayers and in particular the middle prayer and stand before Allah, devoutly obedient.",
        "lang": "en",
        "ref": "Quran 2:238",
    },
    # === Quran: Ayat al-Kursi (negative control — tawhid) ===
    {
        "id": "q_2_255_ar",
        "text": "ٱللَّهُ لَآ إِلَٰهَ إِلَّا هُوَ ٱلْحَىُّ ٱلْقَيُّومُ لَا تَأْخُذُهُۥ سِنَةٌ وَلَا نَوْمٌ لَّهُۥ مَا فِى ٱلسَّمَٰوَٰتِ وَمَا فِى ٱلْأَرْضِ مَن ذَا ٱلَّذِى يَشْفَعُ عِندَهُۥٓ إِلَّا بِإِذْنِهِۦ يَعْلَمُ مَا بَيْنَ أَيْدِيهِمْ وَمَا خَلْفَهُمْ وَلَا يُحِيطُونَ بِشَىْءٍ مِّنْ عِلْمِهِۦٓ إِلَّا بِمَا شَآءَ وَسِعَ كُرْسِيُّهُ ٱلسَّمَٰوَٰتِ وَٱلْأَرْضَ وَلَا يَـُٔودُهُۥ حِفْظُهُمَا وَهُوَ ٱلْعَلِىُّ ٱلْعَظِيمُ",
        "lang": "ar",
        "ref": "Quran 2:255",
    },
    {
        "id": "q_2_255_en",
        "text": "Allah - there is no deity except Him, the Ever-Living, the Sustainer of all existence. Neither drowsiness overtakes Him nor sleep. To Him belongs whatever is in the heavens and whatever is on the earth. Who is it that can intercede with Him except by His permission? He knows what is presently before them and what will be after them, and they encompass not a thing of His knowledge except for what He wills. His Kursi extends over the heavens and the earth, and their preservation tires Him not. And He is the Most High, the Most Great.",
        "lang": "en",
        "ref": "Quran 2:255",
    },
    # === Quran: Riba / Usury (negative control) ===
    {
        "id": "q_2_275_ar",
        "text": "ٱلَّذِينَ يَأْكُلُونَ ٱلرِّبَوٰا۟ لَا يَقُومُونَ إِلَّا كَمَا يَقُومُ ٱلَّذِى يَتَخَبَّطُهُ ٱلشَّيْطَٰنُ مِنَ ٱلْمَسِّ ذَٰلِكَ بِأَنَّهُمْ قَالُوٓا۟ إِنَّمَا ٱلْبَيْعُ مِثْلُ ٱلرِّبَوٰا۟ وَأَحَلَّ ٱللَّهُ ٱلْبَيْعَ وَحَرَّمَ ٱلرِّبَوٰا۟ فَمَن جَآءَهُۥ مَوْعِظَةٌ مِّن رَّبِّهِۦ فَٱنتَهَىٰ فَلَهُۥ مَا سَلَفَ وَأَمْرُهُۥٓ إِلَى ٱللَّهِ وَمَنْ عَادَ فَأُو۟لَٰٓئِكَ أَصْحَٰبُ ٱلنَّارِ هُمْ فِيهَا خَٰلِدُونَ",
        "lang": "ar",
        "ref": "Quran 2:275",
    },
    {
        "id": "q_2_275_en",
        "text": 'Those who consume interest cannot stand on the Day of Resurrection except as one stands who is being beaten by Satan into insanity. That is because they say, "Trade is just like interest." But Allah has permitted trade and has forbidden interest. So whoever has received an admonition from his Lord and desists may have what is past, and his affair rests with Allah. But whoever returns to dealing in interest or usury - those are the companions of the Fire; they will abide eternally therein.',
        "lang": "en",
        "ref": "Quran 2:275",
    },
    # === Quran: Equality of mankind (negative control) ===
    {
        "id": "q_49_13_ar",
        "text": "يَٰٓأَيُّهَا ٱلنَّاسُ إِنَّا خَلَقْنَٰكُم مِّن ذَكَرٍ وَأُنثَىٰ وَجَعَلْنَٰكُمْ شُعُوبًا وَقَبَآئِلَ لِتَعَارَفُوٓا۟ إِنَّ أَكْرَمَكُمْ عِندَ ٱللَّهِ أَتْقَىٰكُمْ إِنَّ ٱللَّهَ عَلِيمٌ خَبِيرٌ",
        "lang": "ar",
        "ref": "Quran 49:13",
    },
    {
        "id": "q_49_13_en",
        "text": "O mankind, indeed We have created you from male and female and made you peoples and tribes that you may know one another. Indeed, the most noble of you in the sight of Allah is the most righteous of you. Indeed, Allah is Knowing and Acquainted.",
        "lang": "en",
        "ref": "Quran 49:13",
    },
    # === Quran: Friday prayer ===
    {
        "id": "q_62_9_ar",
        "text": "يَٰٓأَيُّهَا ٱلَّذِينَ ءَامَنُوٓا۟ إِذَا نُودِىَ لِلصَّلَوٰةِ مِن يَوْمِ ٱلْجُمُعَةِ فَٱسْعَوْا۟ إِلَىٰ ذِكْرِ ٱللَّهِ وَذَرُوا۟ ٱلْبَيْعَ ذَٰلِكُمْ خَيْرٌ لَّكُمْ إِن كُنتُمْ تَعْلَمُونَ",
        "lang": "ar",
        "ref": "Quran 62:9",
    },
    {
        "id": "q_62_9_en",
        "text": "O you who have believed, when the adhan is called for the prayer on the day of Jumuah Friday, then proceed to the remembrance of Allah and leave trade. That is better for you, if you only knew.",
        "lang": "en",
        "ref": "Quran 62:9",
    },
    # === Hadith: Eid days ===
    {
        "id": "h_eid_ar",
        "text": "قدم النبي صلى الله عليه وسلم المدينة ولهم يومان يلعبون فيهما فقال إن الله قد أبدلكم بهما خيرا منهما يوم الأضحى ويوم الفطر",
        "lang": "ar",
        "ref": "Abu Dawud 1134",
    },
    {
        "id": "h_eid_en",
        "text": "The Prophet came to Madinah and the people had two days on which they engaged in games. He said: Allah has substituted for them something better than them, the day of sacrifice (Eid al-Adha) and the day of the breaking of the fast (Eid al-Fitr).",
        "lang": "en",
        "ref": "Abu Dawud 1134",
    },
    # === Hadith: Fasting Ramadan ===
    {
        "id": "h_fasting_ar",
        "text": "من صام رمضان إيمانا واحتسابا غفر له ما تقدم من ذنبه",
        "lang": "ar",
        "ref": "Bukhari 38",
    },
    {
        "id": "h_fasting_en",
        "text": "Whoever fasts during Ramadan out of sincere faith and hoping to attain Allah's rewards, then all his past sins will be forgiven.",
        "lang": "en",
        "ref": "Bukhari 38",
    },
    # === Hadith: Five pillars ===
    {
        "id": "h_prayer_ar",
        "text": "بني الإسلام على خمس شهادة أن لا إله إلا الله وأن محمدا رسول الله وإقام الصلاة وإيتاء الزكاة وحج البيت وصوم رمضان",
        "lang": "ar",
        "ref": "Bukhari 8",
    },
    {
        "id": "h_prayer_en",
        "text": "Islam is built upon five: testifying that there is no god but Allah and that Muhammad is the Messenger of Allah, establishing prayer, giving zakah, making pilgrimage to the House, and fasting Ramadan.",
        "lang": "en",
        "ref": "Bukhari 8",
    },
    # === Hadith: Ihsan ===
    {
        "id": "h_ihsan_ar",
        "text": "أن تعبد الله كأنك تراه فإن لم تكن تراه فإنه يراك",
        "lang": "ar",
        "ref": "Muslim 8",
    },
    # === Hadith: Riba curse ===
    {
        "id": "h_riba_ar",
        "text": "لعن رسول الله صلى الله عليه وسلم آكل الربا ومؤكله وكاتبه وشاهديه",
        "lang": "ar",
        "ref": "Muslim 1598",
    },
    # === Hadith: Friday prayer ===
    {
        "id": "h_jummah_ar",
        "text": "من اغتسل يوم الجمعة غسل الجنابة ثم راح فكأنما قرب بدنة",
        "lang": "ar",
        "ref": "Bukhari 881",
    },
]

# ──────────────────────────────────────────────────────────────────────
# Test queries with ground truth expected matches
#
# Each query simulates what a user would type in a search box. The
# "expected" list contains corpus IDs that YOU (the human) decided are
# the correct answers ahead of time.
#
# Example:
#   query: "الصيام"  (fasting)
#   expected: ["q_2_183_ar", "q_2_185_ar", "h_fasting_ar"]
#
# This says: "If someone searches 'الصيام', a good model should return
# these 3 passages near the top, because they are all about fasting."
#
# What's NOT in expected matters too — q_2_275_ar (usury) and q_2_255_ar
# (Ayat al-Kursi) have nothing to do with fasting, so a good model
# should rank them low.
#
# Scoring walkthrough for query "الصيام":
#   Say a model ranks passages like this:
#     Rank 1: q_2_183_ar  (0.91) ← fasting verse      ✓ expected
#     Rank 2: q_2_185_ar  (0.88) ← Ramadan verse      ✓ expected
#     Rank 3: q_2_275_ar  (0.85) ← usury verse        ✗ not expected
#     Rank 4: h_fasting_ar(0.83) ← fasting hadith     ✓ expected
#
#   MRR:      first hit at rank 1 → 1/1 = 1.0 (perfect)
#   Recall@1: 1 of 3 expected in top 1 → 33%
#   Recall@3: 2 of 3 expected in top 3 → 67% (usury snuck in at rank 3)
#   Recall@5: 3 of 3 expected in top 5 → 100%
#
#   A bad model might score everything ~0.71-0.72 (low spread) and put
#   the fasting verse at rank 3 → MRR = 1/3 = 0.33 (poor).
# ──────────────────────────────────────────────────────────────────────

QUERIES = [
    {
        "query": "عيد",
        "expected": ["q_5_114_ar", "h_eid_ar"],
        "desc": "Short Arabic: Eid (original failure case)",
    },
    {
        "query": "Eid festival",
        "expected": ["q_5_114_en", "h_eid_en"],
        "desc": "English: Eid festival",
    },
    {
        "query": "الصيام",
        "expected": ["q_2_183_ar", "q_2_185_ar", "h_fasting_ar"],
        "desc": "Short Arabic: fasting",
    },
    {
        "query": "fasting",
        "expected": ["q_2_183_en", "q_2_185_en", "h_fasting_en"],
        "desc": "English: fasting",
    },
    {
        "query": "صلاة",
        "expected": ["q_2_43_ar", "q_2_238_ar", "h_prayer_ar", "h_jummah_ar"],
        "desc": "Short Arabic: prayer",
    },
    {
        "query": "prayer",
        "expected": ["q_2_43_en", "q_2_238_en", "h_prayer_en"],
        "desc": "English: prayer",
    },
    {
        "query": "الربا",
        "expected": ["q_2_275_ar", "h_riba_ar"],
        "desc": "Short Arabic: usury/riba",
    },
    {
        "query": "usury interest forbidden",
        "expected": ["q_2_275_en", "h_riba_ar"],
        "desc": "English phrase: usury (cross-lingual)",
    },
    {
        "query": "شهر رمضان",
        "expected": ["q_2_185_ar", "h_fasting_ar"],
        "desc": "Arabic phrase: month of Ramadan",
    },
    {
        "query": "أركان الإسلام",
        "expected": ["h_prayer_ar"],
        "desc": "Arabic concept: pillars of Islam",
    },
    {
        "query": "pillars of Islam",
        "expected": ["h_prayer_en", "h_prayer_ar"],
        "desc": "English concept: pillars of Islam (cross-lingual)",
    },
    {
        "query": "يوم الجمعة",
        "expected": ["h_jummah_ar", "q_62_9_ar"],
        "desc": "Arabic: Friday / Jumu'ah",
    },
    {
        "query": "equality among people",
        "expected": ["q_49_13_en"],
        "desc": "English concept: equality (cross-lingual)",
    },
    {
        "query": "الإحسان في العبادة",
        "expected": ["h_ihsan_ar"],
        "desc": "Arabic phrase: excellence in worship",
    },
    {
        "query": "كتب عليكم الصيام",
        "expected": ["q_2_183_ar"],
        "desc": "Arabic: near-verbatim Quran quote on fasting",
    },
]

# ──────────────────────────────────────────────────────────────────────
# Benchmark engine
# ──────────────────────────────────────────────────────────────────────


@dataclass
class QueryResult:
    query: str
    desc: str
    expected: list
    ranked: list  # list of (id, score, ref)
    mrr: float = 0.0
    recall_at: dict = field(default_factory=dict)


@dataclass
class ModelResult:
    name: str
    query_results: list
    mrr: float = 0.0
    recall_at_1: float = 0.0
    recall_at_3: float = 0.0
    recall_at_5: float = 0.0
    score_spread: float = 0.0
    score_gap: float = 0.0
    encode_time: float = 0.0


def run_benchmark(model_name: str, config: dict) -> ModelResult:
    from sentence_transformers import SentenceTransformer

    print(f"\n{'='*60}")
    print(f"  Loading: {model_name}")
    print(f"  HuggingFace: {config['model_id']}")
    print(f"{'='*60}")

    model = SentenceTransformer(config["model_id"])

    # Encode corpus with passage prefix
    corpus_texts = [config["passage_prefix"] + p["text"] for p in CORPUS]
    corpus_ids = [p["id"] for p in CORPUS]
    corpus_refs = [p["ref"] for p in CORPUS]

    t0 = time.time()
    corpus_embeddings = model.encode(
        corpus_texts, normalize_embeddings=True, show_progress_bar=False
    )
    encode_time = time.time() - t0
    print(f"  Corpus encoded in {encode_time:.2f}s ({len(CORPUS)} passages)")

    # Encode queries with query prefix
    query_texts = [config["query_prefix"] + q["query"] for q in QUERIES]
    query_embeddings = model.encode(
        query_texts, normalize_embeddings=True, show_progress_bar=False
    )

    # Compute similarities and rank
    # Since embeddings are L2-normalized, dot product = cosine similarity
    similarities = np.dot(query_embeddings, corpus_embeddings.T)

    query_results = []
    all_spreads = []
    all_gaps = []

    for i, q in enumerate(QUERIES):
        scores = similarities[i]
        ranked_indices = np.argsort(scores)[::-1]

        ranked = [
            (corpus_ids[idx], float(scores[idx]), corpus_refs[idx])
            for idx in ranked_indices
        ]

        # MRR: reciprocal rank of first expected hit
        mrr = 0.0
        for rank, (cid, _, _) in enumerate(ranked, 1):
            if cid in q["expected"]:
                mrr = 1.0 / rank
                break

        # Recall@K
        recall_at = {}
        for k in [1, 3, 5]:
            top_k_ids = {r[0] for r in ranked[:k]}
            hits = len(set(q["expected"]) & top_k_ids)
            recall_at[k] = hits / len(q["expected"])

        # Score spread (std dev across all corpus passages)
        all_spreads.append(float(np.std(scores)))

        # Score gap (top-1 minus top-5)
        if len(ranked) >= 5:
            all_gaps.append(ranked[0][1] - ranked[4][1])

        query_results.append(
            QueryResult(
                query=q["query"],
                desc=q["desc"],
                expected=q["expected"],
                ranked=ranked,
                mrr=mrr,
                recall_at=recall_at,
            )
        )

    # Aggregate metrics
    result = ModelResult(
        name=model_name,
        query_results=query_results,
        mrr=np.mean([qr.mrr for qr in query_results]),
        recall_at_1=np.mean([qr.recall_at[1] for qr in query_results]),
        recall_at_3=np.mean([qr.recall_at[3] for qr in query_results]),
        recall_at_5=np.mean([qr.recall_at[5] for qr in query_results]),
        score_spread=np.mean(all_spreads),
        score_gap=np.mean(all_gaps) if all_gaps else 0.0,
        encode_time=encode_time,
    )

    # Free memory
    del model
    gc.collect()
    try:
        import torch
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
    except ImportError:
        pass

    return result


# ──────────────────────────────────────────────────────────────────────
# Output formatting
# ──────────────────────────────────────────────────────────────────────


def print_query_details(result: ModelResult):
    """Print per-query top-5 breakdown for a model."""
    print(f"\n  Model: {result.name}")
    print(f"  {'─'*56}")

    for qr in result.query_results:
        print(f"\n  Query: \"{qr.query}\"  ({qr.desc})")
        expected_set = set(qr.expected)
        for rank, (cid, score, ref) in enumerate(qr.ranked[:5], 1):
            hit = "✓" if cid in expected_set else " "
            # Truncate text for display
            text = ""
            for p in CORPUS:
                if p["id"] == cid:
                    text = p["text"][:60]
                    break
            print(f"    #{rank} [{score:+.4f}] {hit} {cid:<20s} {ref:<16s} {text}...")

        # Show recall
        for k in [1, 3, 5]:
            top_k_ids = {r[0] for r in qr.ranked[:k]}
            hits = len(expected_set & top_k_ids)
            print(f"    Recall@{k}: {hits}/{len(qr.expected)}", end="  ")
        print()


def print_summary_table(results: list):
    """Print comparison table across all models."""
    print(f"\n{'='*100}")
    print("  SUMMARY: Arabic Embedding Model Benchmark")
    print(f"{'='*100}")
    print()

    # Header
    header = f"  {'Model':<35s} {'MRR':>6s} {'R@1':>6s} {'R@3':>6s} {'R@5':>6s} {'Spread':>8s} {'Gap':>8s} {'Time':>6s}"
    print(header)
    print(f"  {'─'*35} {'─'*6} {'─'*6} {'─'*6} {'─'*6} {'─'*8} {'─'*8} {'─'*6}")

    # Sort by MRR descending
    for r in sorted(results, key=lambda x: x.mrr, reverse=True):
        print(
            f"  {r.name:<35s} {r.mrr:>6.3f} {r.recall_at_1:>5.1%} {r.recall_at_3:>5.1%} "
            f"{r.recall_at_5:>5.1%} {r.score_spread:>8.4f} {r.score_gap:>8.4f} {r.encode_time:>5.1f}s"
        )

    print()
    print("  MRR     = Mean Reciprocal Rank (higher = first correct result ranks higher)")
    print("  R@K     = Recall at K (% of expected results found in top K)")
    print("  Spread  = Avg std dev of cosine scores (higher = better discrimination)")
    print("  Gap     = Avg score difference between #1 and #5 (higher = better separation)")
    print()

    # Highlight winner
    best = max(results, key=lambda x: x.mrr)
    print(f"  Winner by MRR: {best.name} ({best.mrr:.3f})")

    # Check for the "everything scores the same" problem
    worst_spread = min(results, key=lambda x: x.score_spread)
    if worst_spread.score_spread < 0.05:
        print(
            f"  ⚠ Warning: {worst_spread.name} has very low score spread ({worst_spread.score_spread:.4f})"
            " — scores are clustered, model cannot discriminate Arabic text"
        )


# ──────────────────────────────────────────────────────────────────────
# Per-query category breakdown
# ──────────────────────────────────────────────────────────────────────


def print_category_breakdown(results: list):
    """Show how each model performs on different query types."""
    categories = {
        "Short Arabic (1 word)": [0, 2, 4, 6],      # عيد, الصيام, صلاة, الربا
        "English single word": [3, 5],                 # fasting, prayer
        "Arabic phrase": [8, 11, 13, 14],             # شهر رمضان, يوم الجمعة, إحسان, كتب عليكم
        "English phrase/concept": [1, 7, 10, 12],     # Eid festival, usury, pillars, equality
        "Concept (not exact words)": [9, 10, 13],     # أركان الإسلام, pillars, إحسان
    }

    print(f"\n{'='*100}")
    print("  BREAKDOWN BY QUERY CATEGORY")
    print(f"{'='*100}")
    print()

    header = f"  {'Category':<30s}"
    for r in sorted(results, key=lambda x: x.mrr, reverse=True):
        header += f" {r.name[:18]:>18s}"
    print(header)
    print(f"  {'─'*30}" + (" " + "─" * 18) * len(results))

    for cat_name, indices in categories.items():
        row = f"  {cat_name:<30s}"
        for r in sorted(results, key=lambda x: x.mrr, reverse=True):
            cat_mrr = np.mean([r.query_results[i].mrr for i in indices])
            row += f" {cat_mrr:>17.3f}"
        print(row)

    print()
    print("  Values show MRR per category (higher is better)")


# ──────────────────────────────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(
        description="Benchmark Arabic embedding models for Islamic text retrieval"
    )
    parser.add_argument(
        "--models",
        type=str,
        default=None,
        help="Comma-separated model names to test (default: all). "
        f"Available: {', '.join(MODEL_CONFIGS.keys())}",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="Show per-query top-5 breakdown"
    )
    args = parser.parse_args()

    # Select models
    if args.models:
        model_names = [m.strip() for m in args.models.split(",")]
        for name in model_names:
            if name not in MODEL_CONFIGS:
                print(f"Unknown model: {name}")
                print(f"Available: {', '.join(MODEL_CONFIGS.keys())}")
                sys.exit(1)
    else:
        model_names = list(MODEL_CONFIGS.keys())

    print(f"Benchmarking {len(model_names)} models on {len(CORPUS)} passages × {len(QUERIES)} queries")
    print(f"Models: {', '.join(model_names)}")

    # Run benchmarks
    results = []
    for name in model_names:
        result = run_benchmark(name, MODEL_CONFIGS[name])
        results.append(result)

        if args.verbose:
            print_query_details(result)

    # Print results
    print_summary_table(results)
    print_category_breakdown(results)


if __name__ == "__main__":
    main()
