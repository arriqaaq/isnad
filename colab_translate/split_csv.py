"""Split sanadset.csv by book into per-book JSON files for Colab translation.

Usage:
    cd colab_translate
    python split_csv.py
"""

import csv
import json
import os
import re

csv.field_size_limit(10_000_000)

CSV_PATH = os.path.join(os.path.dirname(__file__), "..", "data", "sanadset.csv")
OUT_DIR = os.path.join(os.path.dirname(__file__), "splits")

# The 6 canonical collections (must match DEFAULT_BOOKS / BOOK_CODES in sanadset.rs)
BOOKS = {
    "صحيح البخاري": "bukhari",
    "صحيح مسلم": "muslim",
    "سنن أبي داود": "abudawud",
    "سنن النسائى الصغرى": "nasai",
    "جامع الترمذي": "tirmidhi",
    "سنن ابن ماجه": "ibnmajah",
}

TAG_RE = re.compile(r"<[^>]+>")


def strip_tags(text: str) -> str:
    return TAG_RE.sub("", text).strip()


def parse_sanad_list(sanad: str) -> list[str]:
    """Parse "['name1', 'name2', ...]" into a list of names."""
    sanad = sanad.strip()
    if sanad in ("No SANAD", ""):
        return []
    # Remove outer brackets and split by quotes
    names = []
    for part in re.findall(r"'([^']+)'", sanad):
        name = strip_tags(part.strip())
        if name:
            names.append(name)
    return names


def main():
    if not os.path.exists(CSV_PATH):
        print(f"Error: {CSV_PATH} not found")
        print("Make sure data/sanadset.csv exists in the project root.")
        return

    os.makedirs(OUT_DIR, exist_ok=True)

    # Collect hadiths and narrators per book
    book_hadiths: dict[str, list] = {slug: [] for slug in BOOKS.values()}
    book_narrators: dict[str, set] = {slug: set() for slug in BOOKS.values()}

    with open(CSV_PATH, encoding="utf-8") as f:
        reader = csv.reader(f)
        next(reader)  # skip header: Hadith, Book, Num_hadith, Matn, Sanad, Sanad_Length

        for row in reader:
            if len(row) < 5:
                continue

            book_ar = row[1]
            slug = BOOKS.get(book_ar)
            if slug is None:
                continue

            try:
                hadith_num = int(row[2])
            except ValueError:
                continue
            if hadith_num == 0:
                continue

            text_ar = strip_tags(row[0])
            matn = strip_tags(row[3])

            # Prefer matn (hadith content without isnad), fall back to full text
            text = matn if matn else text_ar
            if not text:
                continue
            text = text[:3000]  # match Rust truncation

            book_hadiths[slug].append({
                "hadith_number": hadith_num,
                "text_ar": text,
                "matn": matn[:3000] if matn else None,
            })

            # Extract narrator names from Sanad column
            for name in parse_sanad_list(row[4]):
                book_narrators[slug].add(name)

    # Write output files
    for slug in BOOKS.values():
        hadiths = book_hadiths[slug]
        narrators = sorted(book_narrators[slug])

        hadith_path = os.path.join(OUT_DIR, f"{slug}.json")
        narrator_path = os.path.join(OUT_DIR, f"{slug}_narrators.json")

        with open(hadith_path, "w", encoding="utf-8") as f:
            json.dump(hadiths, f, ensure_ascii=False, indent=1)

        narrator_list = [{"name_ar": n} for n in narrators]
        with open(narrator_path, "w", encoding="utf-8") as f:
            json.dump(narrator_list, f, ensure_ascii=False, indent=1)

        print(f"  {slug:12s}  {len(hadiths):>6,} hadiths  {len(narrators):>5,} narrators")

    total_h = sum(len(v) for v in book_hadiths.values())
    total_n = sum(len(v) for v in book_narrators.values())
    print(f"\n  Total: {total_h:,} hadiths, {total_n:,} narrators")
    print(f"  Output: {OUT_DIR}/")


if __name__ == "__main__":
    main()
