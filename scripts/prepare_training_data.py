#!/usr/bin/env python3
"""
Generate instruction-tuning data for the hadith-scholar model.

Parses raw CSV files directly (no SurrealDB dependency):
  - data/sanadset.csv        (source of truth: Arabic text + narrator chains)
  - data/translations/*.csv  (enrichment: English translations + chapter titles)
  - data/quran.csv           (Quran verses + Tafsir Ibn Kathir)

Uses Ollama for hadith Q&A generation only (~400 calls).
Tafsir and chain analysis examples are built from templates (no Ollama needed).

Prerequisites:
  - Ollama running locally with llama3.2 (only for hadith Q&A category)
  - data/quran.csv (run: python3 scripts/prepare_quran_data.py)

Output:
  - data/train.jsonl
  - data/valid.jsonl

Usage:
  python3 scripts/prepare_training_data.py [--ollama-url URL] [--model MODEL] [--workers N]
"""

import ast
import csv
import json
import os
import random
import re
import sys
import time
import argparse
import urllib.request
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed

try:
    from tqdm import tqdm
except ImportError:
    # Minimal fallback if tqdm not installed
    class tqdm:
        def __init__(self, iterable=None, total=None, desc="", **kwargs):
            self.iterable = iterable
            self.total = total
            self.desc = desc
            self.n = 0
        def __iter__(self):
            for item in self.iterable:
                yield item
                self.n += 1
                if self.n % 25 == 0 or self.n == self.total:
                    print(f"\r  {self.desc}: {self.n}/{self.total}", end="", flush=True)
            print()
        def update(self, n=1):
            self.n += n
            if self.n % 10 == 0 or self.n == self.total:
                print(f"\r  {self.desc}: {self.n}/{self.total}", end="", flush=True)
        def close(self):
            print()

# Sanadset has very large fields (Arabic text with full isnad)
csv.field_size_limit(sys.maxsize)

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data")
SANADSET_PATH = os.path.join(DATA_DIR, "sanadset.csv")
TRANSLATIONS_DIR = os.path.join(DATA_DIR, "translations")
QURAN_PATH = os.path.join(DATA_DIR, "quran.csv")
TRAIN_OUTPUT = os.path.join(DATA_DIR, "train.jsonl")
VALID_OUTPUT = os.path.join(DATA_DIR, "valid.jsonl")

DEFAULT_OLLAMA_URL = "http://localhost:11434"
DEFAULT_MODEL = "llama3.2"

# The 6 major books (Kutub al-Sittah) — Arabic names matching sanadset.csv
# Note: Tirmidhi appears as "جامع الترمذي" in sanadset, not "سنن الترمذي"
MAJOR_BOOKS = {
    "صحيح البخاري": "bukhari",
    "صحيح مسلم": "muslim",
    "سنن أبي داود": "abudawud",
    "سنن النسائى الصغرى": "nasai",
    "جامع الترمذي": "tirmidhi",
    "سنن ابن ماجه": "ibnmajah",
}

BOOK_ENGLISH = {
    "bukhari": "Sahih al-Bukhari",
    "muslim": "Sahih Muslim",
    "abudawud": "Sunan Abi Dawud",
    "nasai": "Sunan an-Nasa'i",
    "tirmidhi": "Jami' at-Tirmidhi",
    "ibnmajah": "Sunan Ibn Majah",
}

# System prompt matching src/rag.rs
SYSTEM_PROMPT_TEMPLATE = (
    "You are a knowledgeable Islamic scholar assistant specializing in hadith sciences.\n"
    "Answer questions using ONLY the hadiths provided below as context.\n"
    "Always cite the hadith number when referencing a hadith.\n"
    "When relevant, mention the chain of narration (isnad) to support authenticity.\n"
    "If the context doesn't contain relevant information, say so honestly.\n"
    "Be concise and accurate.\n\n"
    "## Relevant Hadiths:\n\n{context}"
)

HADITH_QUESTION_TEMPLATES = [
    "What do these hadiths teach about {topic}?",
    "Explain the guidance from the Prophet regarding {topic}.",
    "What is the Islamic teaching on {topic} based on these narrations?",
    "Summarize what these hadiths say about {topic}.",
    "What can we learn about {topic} from these hadiths?",
    "Based on these narrations, what is the ruling or guidance on {topic}?",
]

CHAIN_QUESTION_TEMPLATES = [
    "How reliable is the chain of narration for this hadith?",
    "Assess the isnad (chain of narration) of this hadith.",
    "What can you tell me about the narrators in this hadith's chain?",
    "Evaluate the transmission chain of this narration.",
]

QURAN_QUESTION_TEMPLATES = [
    "What is the meaning of this verse?",
    "Explain the tafsir (interpretation) of this Quranic verse.",
    "What does this verse teach us?",
    "Provide the scholarly interpretation of this verse.",
]


# ---------------------------------------------------------------------------
# XML tag stripping (matching src/ingest/sanadset.rs strip_tags)
# ---------------------------------------------------------------------------

TAG_RE = re.compile(r"</?(?:SANAD|MATN|NAR|IDF)[^>]*>")
HTML_TAG_RE = re.compile(r"<[^>]+>")


def strip_tags(text: str) -> str:
    return TAG_RE.sub("", text).strip()


def strip_html(text: str) -> str:
    """Strip HTML tags from tafsir text."""
    return HTML_TAG_RE.sub("", text).strip()


# ---------------------------------------------------------------------------
# Parse sanadset.csv
# ---------------------------------------------------------------------------

def parse_sanad_list(raw: str) -> list[str]:
    """Parse narrator chain from Python list string format."""
    raw = raw.strip()
    if not raw or raw == "No SANAD":
        return []
    try:
        parsed = ast.literal_eval(raw)
        if isinstance(parsed, list):
            return [strip_tags(str(n)).strip() for n in parsed if str(n).strip()]
    except (ValueError, SyntaxError):
        pass
    return []


def load_sanadset() -> dict[str, list[dict]]:
    """Load sanadset.csv, return hadiths grouped by book code."""
    print("Loading sanadset.csv...")
    if not os.path.exists(SANADSET_PATH):
        print(f"  ERROR: {SANADSET_PATH} not found. Run 'make ingest' first.")
        sys.exit(1)

    hadiths_by_book: dict[str, list[dict]] = defaultdict(list)
    seen: set[tuple[str, int]] = set()

    with open(SANADSET_PATH, encoding="utf-8") as f:
        reader = csv.reader(f)
        next(reader)
        for row in reader:
            if len(row) < 6:
                continue
            book_ar = row[1].strip()
            if book_ar not in MAJOR_BOOKS:
                continue
            book_code = MAJOR_BOOKS[book_ar]
            try:
                num = int(row[2])
            except ValueError:
                continue
            if num == 0:
                continue
            key = (book_code, num)
            if key in seen:
                continue
            seen.add(key)

            hadiths_by_book[book_code].append({
                "num": num,
                "text_ar": strip_tags(row[0].strip()),
                "matn": strip_tags(row[3].strip()),
                "chain": parse_sanad_list(row[4]),
            })

    total = sum(len(v) for v in hadiths_by_book.values())
    for code, hs in sorted(hadiths_by_book.items()):
        print(f"  {BOOK_ENGLISH.get(code, code)}: {len(hs)} hadiths")
    print(f"  Total: {total}")
    return dict(hadiths_by_book)


def load_translations() -> dict[str, dict[int, dict]]:
    """Load translation CSVs, return lookup by book_code -> hadith_num -> data."""
    print("Loading translations...")
    translations: dict[str, dict[int, dict]] = {}

    if not os.path.isdir(TRANSLATIONS_DIR):
        print(f"  WARNING: {TRANSLATIONS_DIR} not found, skipping translations")
        return translations

    for book_code in MAJOR_BOOKS.values():
        csv_path = os.path.join(TRANSLATIONS_DIR, f"{book_code}.csv")
        if not os.path.exists(csv_path):
            continue
        lookup: dict[int, dict] = {}
        with open(csv_path, encoding="utf-8-sig") as f:
            reader = csv.DictReader(f)
            for row in reader:
                ref = row.get("Reference", "")
                match = re.search(r":(\d+)$", ref)
                if not match:
                    continue
                num = int(match.group(1))
                lookup[num] = {
                    "text_en": row.get("English_Text", "").strip(),
                    "chapter": row.get("Chapter_Title_English", "").strip(),
                    "grade": row.get("Grade", "").strip(),
                }
        translations[book_code] = lookup
        print(f"  {BOOK_ENGLISH.get(book_code, book_code)}: {len(lookup)} translations")

    return translations


def enrich_hadiths(
    hadiths_by_book: dict[str, list[dict]],
    translations: dict[str, dict[int, dict]],
) -> dict[str, list[dict]]:
    """Merge translation data into sanadset hadiths."""
    print("Enriching hadiths with translations...")
    enriched_count = 0
    total = 0
    for book_code, hadiths in hadiths_by_book.items():
        lookup = translations.get(book_code, {})
        for h in hadiths:
            total += 1
            tr = lookup.get(h["num"])
            if tr and tr["text_en"]:
                h["text_en"] = tr["text_en"]
                h["chapter"] = tr.get("chapter", "")
                h["grade"] = tr.get("grade", "")
                enriched_count += 1
    print(f"  Enriched {enriched_count}/{total} hadiths with English translations")
    return hadiths_by_book


def load_quran() -> list[dict]:
    """Load quran.csv, return list of verses with non-empty tafsir."""
    print("Loading quran.csv...")
    if not os.path.exists(QURAN_PATH):
        print(f"  WARNING: {QURAN_PATH} not found. Run: python3 scripts/prepare_quran_data.py")
        return []

    verses = []
    with open(QURAN_PATH, encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            tafsir = row.get("tafsir_en", "").strip()
            if not tafsir:
                continue
            verses.append({
                "surah": int(row["surah"]),
                "ayah": int(row["ayah"]),
                "text_ar": row.get("text_ar", "").strip(),
                "text_en": row.get("text_en", "").strip(),
                "tafsir_en": tafsir,
            })
    print(f"  {len(verses)} verses with tafsir loaded")
    return verses


# ---------------------------------------------------------------------------
# Ollama helper
# ---------------------------------------------------------------------------

def ollama_generate(
    prompt: str,
    system: str,
    ollama_url: str,
    model: str,
) -> str:
    """Call Ollama chat API (non-streaming) and return the assistant response."""
    payload = json.dumps({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt},
        ],
        "stream": False,
        "options": {"temperature": 0.7, "num_predict": 300},
    }).encode("utf-8")

    req = urllib.request.Request(
        f"{ollama_url}/api/chat",
        data=payload,
        headers={"Content-Type": "application/json"},
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data.get("message", {}).get("content", "").strip()
    except Exception as e:
        return ""


# ---------------------------------------------------------------------------
# Format hadith context (matching rag.rs)
# ---------------------------------------------------------------------------

def format_hadith_context(hadiths: list[dict]) -> str:
    """Format hadiths into the context block matching rag.rs."""
    parts = []
    for h in hadiths:
        narrator = h["chain"][-1] if h.get("chain") else "Unknown narrator"
        line = f"Hadith #{h['num']} — {narrator}\n"
        if h.get("chain") and len(h["chain"]) > 1:
            chain_str = " → ".join(h["chain"])
            line += f"Chain of narration: {chain_str}\n"
        text = h.get("text_en") or h.get("matn") or h.get("text_ar", "")
        line += f"{text}\n"
        parts.append(line)
    return "\n".join(parts)


# ---------------------------------------------------------------------------
# Category 1: Hadith RAG Q&A (uses Ollama, parallelized)
# ---------------------------------------------------------------------------

def _build_hadith_task(book_code, chapter, hs):
    """Build a single hadith Q&A task (context + question). No Ollama call yet."""
    sample_size = min(random.randint(2, 4), len(hs))
    sampled = random.sample(hs, sample_size)
    context = format_hadith_context(sampled)
    system_prompt = SYSTEM_PROMPT_TEMPLATE.format(context=context)
    topic = re.sub(r"^Chapter:\s*", "", chapter).strip()
    if not topic:
        return None
    question = random.choice(HADITH_QUESTION_TEMPLATES).format(topic=topic)
    return {
        "system_prompt": system_prompt,
        "context": context,
        "question": question,
    }


def _generate_one_hadith_qa(task, ollama_url, model):
    """Generate a single hadith Q&A example via Ollama."""
    gen_system = (
        "You are a knowledgeable Islamic scholar. Answer the question using ONLY "
        "the hadiths provided. Always cite hadith numbers (e.g., Hadith #123). "
        "Mention the chain of narration when available. Be concise (150-300 words)."
    )
    gen_prompt = (
        f"Context:\n{task['context']}\n\nQuestion: {task['question']}\n\n"
        "Provide a scholarly answer citing the hadith numbers and narrators."
    )
    answer = ollama_generate(gen_prompt, gen_system, ollama_url, model)
    if not answer:
        return None
    # Validate: answer must cite at least one hadith number
    if not re.search(r"#?\d+", answer):
        return None
    return {
        "messages": [
            {"role": "system", "content": task["system_prompt"]},
            {"role": "user", "content": task["question"]},
            {"role": "assistant", "content": answer},
        ]
    }


def generate_hadith_qa(
    hadiths_by_book: dict[str, list[dict]],
    ollama_url: str,
    model: str,
    target_count: int = 400,
    workers: int = 4,
) -> list[dict]:
    """Generate hadith Q&A examples via parallel Ollama calls."""
    print(f"\nGenerating Category 1: Hadith RAG Q&A (target: {target_count}, workers: {workers})...")

    # Group hadiths by chapter
    chapter_groups = []
    for book_code, hadiths in hadiths_by_book.items():
        by_chapter: dict[str, list[dict]] = defaultdict(list)
        for h in hadiths:
            if h.get("chapter") and h.get("text_en"):
                by_chapter[h["chapter"]].append(h)
        for chapter, hs in by_chapter.items():
            if len(hs) >= 2:
                chapter_groups.append((book_code, chapter, hs))

    random.shuffle(chapter_groups)
    print(f"  Found {len(chapter_groups)} chapter groups")

    # Pre-build all tasks (no Ollama calls)
    tasks = []
    for book_code, chapter, hs in chapter_groups:
        if len(tasks) >= target_count + 50:  # overshoot slightly for failures
            break
        task = _build_hadith_task(book_code, chapter, hs)
        if task:
            tasks.append(task)

    # Run Ollama calls in parallel
    examples = []
    pbar = tqdm(total=min(len(tasks), target_count), desc="  Hadith Q&A (Ollama)")
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {
            pool.submit(_generate_one_hadith_qa, t, ollama_url, model): t
            for t in tasks
        }
        for future in as_completed(futures):
            result = future.result()
            if result:
                examples.append(result)
                pbar.update(1)
            if len(examples) >= target_count:
                for f in futures:
                    f.cancel()
                break
    pbar.close()

    print(f"  Done: {len(examples)} hadith Q&A examples")
    return examples[:target_count]


# ---------------------------------------------------------------------------
# Category 2: Quran Tafsir Q&A (NO Ollama — uses tafsir text directly)
# ---------------------------------------------------------------------------

def _truncate_tafsir(tafsir: str, max_words: int = 500) -> str:
    """Truncate long tafsir to max_words, ending at sentence boundary."""
    words = tafsir.split()
    if len(words) <= max_words:
        return tafsir
    truncated = " ".join(words[:max_words])
    # Try to end at a sentence boundary
    last_period = truncated.rfind(".")
    if last_period > len(truncated) // 2:
        truncated = truncated[: last_period + 1]
    return truncated


def generate_quran_qa(
    verses: list[dict],
    target_count: int = 400,
) -> list[dict]:
    """Generate Quran tafsir Q&A examples using tafsir text directly (no Ollama)."""
    print(f"\nGenerating Category 2: Quran Tafsir Q&A (target: {target_count}, no Ollama)...")
    examples = []

    random.shuffle(verses)

    pbar = tqdm(total=target_count, desc="  Tafsir Q&A")
    for verse in verses:
        if len(examples) >= target_count:
            break

        tafsir = verse["tafsir_en"]
        # Skip very short tafsir (not useful for training)
        if len(tafsir.split()) < 30:
            continue

        context = (
            f"Surah {verse['surah']}, Ayah {verse['ayah']}:\n"
            f"Arabic: {verse['text_ar']}\n"
            f"English: {verse['text_en']}\n"
        )

        system_prompt = (
            "You are a knowledgeable Islamic scholar assistant specializing in "
            "Quranic exegesis (tafsir).\n"
            "Answer questions using the provided verse and scholarly interpretation.\n"
            "Always reference the surah and ayah number.\n"
            "Be concise and accurate.\n\n"
            f"## Quranic Verse:\n\n{context}"
        )

        question = random.choice(QURAN_QUESTION_TEMPLATES)

        # Use tafsir directly as the answer — it IS the scholarly interpretation
        # Strip HTML tags that leak from the source data
        answer = _truncate_tafsir(strip_html(tafsir))
        # Prepend a reference line so the model learns to cite
        answer = (
            f"Regarding Surah {verse['surah']}, Ayah {verse['ayah']} — "
            f"\"{verse['text_en'][:100]}{'...' if len(verse['text_en']) > 100 else ''}\"\n\n"
            f"{answer}"
        )

        examples.append({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
                {"role": "assistant", "content": answer},
            ]
        })
        pbar.update(1)

    pbar.close()
    print(f"  Done: {len(examples)} tafsir Q&A examples (instant, no Ollama)")
    return examples


# ---------------------------------------------------------------------------
# Category 3: Chain Analysis (NO Ollama — template-based)
# ---------------------------------------------------------------------------

def _build_chain_answer(book_code: str, h: dict) -> str:
    """Build a chain analysis answer from the hadith data using templates."""
    chain = h["chain"]
    num = h["num"]
    book_en = BOOK_ENGLISH.get(book_code, book_code)
    chain_len = len(chain)

    # Primary narrator (closest to Prophet) is last in chain
    primary = chain[-1] if chain else "Unknown"
    # Collector (closest to author) is first in chain
    collector = chain[0] if chain else "Unknown"

    chain_str = " → ".join(chain)

    # Chain length classification
    if chain_len <= 3:
        length_note = f"This is a relatively short chain ({chain_len} narrators), which is generally considered stronger as there are fewer links where errors could occur."
    elif chain_len <= 5:
        length_note = f"This chain has {chain_len} narrators, which is a moderate length for a hadith chain."
    else:
        length_note = f"This is a longer chain with {chain_len} narrators. Longer chains require careful verification of each link."

    answer = (
        f"Hadith #{num} from {book_en} has the following chain of narration (isnad):\n\n"
        f"{chain_str}\n\n"
        f"The hadith was collected through {collector} and traces back to {primary} "
        f"as the primary narrator. {length_note}\n\n"
        f"The chain consists of {chain_len} narrators. "
        f"Being recorded in {book_en}, this narration has been subject to the "
        f"rigorous authentication standards of the compiler."
    )
    return answer


def generate_chain_analysis(
    hadiths_by_book: dict[str, list[dict]],
    target_count: int = 200,
) -> list[dict]:
    """Generate chain analysis examples from templates (no Ollama)."""
    print(f"\nGenerating Category 3: Chain Analysis (target: {target_count}, no Ollama)...")
    examples = []

    candidates = []
    for book_code, hadiths in hadiths_by_book.items():
        for h in hadiths:
            if h.get("chain") and len(h["chain"]) >= 3 and h.get("text_en"):
                candidates.append((book_code, h))

    random.shuffle(candidates)
    print(f"  Found {len(candidates)} hadiths with 3+ narrator chains")

    pbar = tqdm(total=target_count, desc="  Chain Analysis")
    for book_code, h in candidates:
        if len(examples) >= target_count:
            break

        context = format_hadith_context([h])
        system_prompt = SYSTEM_PROMPT_TEMPLATE.format(context=context)
        question = random.choice(CHAIN_QUESTION_TEMPLATES)
        answer = _build_chain_answer(book_code, h)

        examples.append({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
                {"role": "assistant", "content": answer},
            ]
        })
        pbar.update(1)

    pbar.close()
    print(f"  Done: {len(examples)} chain analysis examples (instant, no Ollama)")
    return examples


# ---------------------------------------------------------------------------
# Write output
# ---------------------------------------------------------------------------

def write_jsonl(examples: list[dict], path: str) -> None:
    with open(path, "w", encoding="utf-8") as f:
        for ex in examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")
    print(f"  Wrote {len(examples)} examples to {path}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Generate training data for hadith-scholar LLM")
    parser.add_argument("--ollama-url", default=DEFAULT_OLLAMA_URL, help="Ollama server URL")
    parser.add_argument("--model", default=DEFAULT_MODEL, help="Ollama model name")
    parser.add_argument("--seed", type=int, default=42, help="Random seed")
    parser.add_argument("--hadith-count", type=int, default=400, help="Target hadith Q&A examples (uses Ollama)")
    parser.add_argument("--quran-count", type=int, default=400, help="Target Quran tafsir examples (no Ollama)")
    parser.add_argument("--chain-count", type=int, default=200, help="Target chain analysis examples (no Ollama)")
    parser.add_argument("--workers", type=int, default=4, help="Parallel Ollama workers")
    parser.add_argument("--split-ratio", type=float, default=0.9, help="Train/valid split ratio")
    args = parser.parse_args()

    random.seed(args.seed)

    # Check Ollama is running
    print(f"Checking Ollama at {args.ollama_url}...")
    try:
        urllib.request.urlopen(f"{args.ollama_url}/api/tags", timeout=5)
        print("  Ollama is running")
    except Exception as e:
        print(f"  ERROR: Cannot reach Ollama at {args.ollama_url}: {e}")
        print("  Start Ollama first: ollama serve")
        sys.exit(1)

    # Step 1: Load data
    t0 = time.time()
    hadiths_by_book = load_sanadset()
    translations = load_translations()
    hadiths_by_book = enrich_hadiths(hadiths_by_book, translations)
    verses = load_quran()
    print(f"  Data loaded in {time.time() - t0:.1f}s")

    # Step 2: Generate examples
    all_examples = []

    # Category 2 & 3: instant (no Ollama)
    if verses:
        quran_examples = generate_quran_qa(verses, args.quran_count)
        all_examples.extend(quran_examples)
    else:
        quran_examples = []

    chain_examples = generate_chain_analysis(hadiths_by_book, args.chain_count)
    all_examples.extend(chain_examples)

    # Category 1: uses Ollama (parallelized)
    t1 = time.time()
    hadith_examples = generate_hadith_qa(
        hadiths_by_book, args.ollama_url, args.model, args.hadith_count, args.workers
    )
    all_examples.extend(hadith_examples)
    print(f"  Ollama generation took {time.time() - t1:.1f}s")

    # Step 3: Shuffle and split
    print(f"\nTotal examples: {len(all_examples)}")
    random.shuffle(all_examples)

    split_idx = int(len(all_examples) * args.split_ratio)
    train_examples = all_examples[:split_idx]
    valid_examples = all_examples[split_idx:]

    # Step 4: Write output
    print("\nWriting output...")
    write_jsonl(train_examples, TRAIN_OUTPUT)
    write_jsonl(valid_examples, VALID_OUTPUT)

    # Step 5: Summary
    total_time = time.time() - t0
    print(f"\nSummary:")
    print(f"  Hadith Q&A (Ollama):  {len(hadith_examples)}")
    print(f"  Quran Tafsir:         {len(quran_examples)}")
    print(f"  Chain Analysis:       {len(chain_examples)}")
    print(f"  Train set:            {len(train_examples)}")
    print(f"  Valid set:            {len(valid_examples)}")
    print(f"  Total time:           {total_time:.0f}s ({total_time/60:.1f}m)")
    print(f"\nOutput files:")
    print(f"  {TRAIN_OUTPUT}")
    print(f"  {VALID_OUTPUT}")


if __name__ == "__main__":
    main()
