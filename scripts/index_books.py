#!/usr/bin/env python3
"""
Build PageIndex tree structures from Turath book data (JSON → Markdown → Tree).

Converts existing Turath pages + headings JSON files into markdown, then uses
PageIndex's md_to_tree() to build hierarchical tree structures. No LLM calls
needed for the basic tree. Optional --with-summaries flag uses Ollama to
generate node summaries.

Requires:
  - PageIndex deps: pip install -r ../PageIndex/requirements.txt
  - Book data downloaded: make turath-fetch (or individual fetch scripts)

Usage:
  python3 scripts/index_books.py                    # build all books
  python3 scripts/index_books.py --book-id 1673     # build one book
  python3 scripts/index_books.py --with-summaries   # include Ollama summaries
  python3 scripts/index_books.py --status            # show build status

Outputs:
  data/markdown/{slug}.md              — intermediate markdown file
  data/pageindex/{book_id}.json        — PageIndex tree structure
  data/pageindex/book_map.json         — book_id → metadata mapping (used by Rust)
"""

import asyncio
import json
import os
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_DIR = SCRIPT_DIR.parent
DATA_DIR = PROJECT_DIR / "data"
MARKDOWN_DIR = DATA_DIR / "markdown"
PAGEINDEX_DIR = DATA_DIR / "pageindex"
BOOK_MAP_FILE = PAGEINDEX_DIR / "book_map.json"

# Add PageIndex to path
PAGEINDEX_PROJECT = PROJECT_DIR.parent / "PageIndex"
sys.path.insert(0, str(PAGEINDEX_PROJECT))

# Book definitions: book_id → file slugs + metadata
BOOKS = [
    {
        "id": 1673,
        "slug": "fath_al_bari",
        "name_en": "Fath al-Bari",
        "name_ar": "فتح الباري بشرح البخاري",
    },
    {
        "id": 23604,
        "slug": "tafsir_ibn_kathir",
        "name_en": "Tafsir Ibn Kathir",
        "name_ar": "تفسير ابن كثير",
    },
    {
        "id": 7798,
        "slug": "tafsir_tabari",
        "name_en": "Tafsir al-Tabari",
        "name_ar": "تفسير الطبري جامع البيان",
    },
    {
        "id": 1711,
        "slug": "nawawi_on_muslim",
        "name_en": "Sharh Nawawi ala Muslim",
        "name_ar": "شرح النووي على مسلم",
    },
    {
        "id": 21662,
        "slug": "tuhfat_ahwadhi",
        "name_en": "Tuhfat al-Ahwadhi",
        "name_ar": "تحفة الأحوذي",
    },
    {
        "id": 1147,
        "slug": "sahih_nasai",
        "name_en": "Sahih Sunan al-Nasai",
        "name_ar": "صحيح سنن النسائي",
    },
    {
        "id": 5760,
        "slug": "awn_mabud",
        "name_en": "Awn al-Mabud",
        "name_ar": "عون المعبود",
    },
    {
        "id": 98138,
        "slug": "ibn_majah",
        "name_en": "Sunan Ibn Majah",
        "name_ar": "سنن ابن ماجه",
    },
    {
        "id": 1278,
        "slug": "tahdhib",
        "name_en": "Tahdhib al-Tahdhib",
        "name_ar": "تهذيب التهذيب",
    },
]


def load_book_data(slug: str) -> tuple[list, list] | None:
    """Load headings and pages JSON for a book. Returns (headings, pages) or None."""
    headings_path = DATA_DIR / f"{slug}_headings.json"
    pages_path = DATA_DIR / f"{slug}_pages.json"

    if not headings_path.exists():
        print(f"  Headings not found: {headings_path.name}")
        return None
    if not pages_path.exists():
        print(f"  Pages not found: {pages_path.name}")
        return None

    with open(headings_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    headings = data.get("indexes", {}).get("headings", [])
    if not headings:
        # Some files have headings at top level
        headings = data.get("headings", [])

    with open(pages_path, "r", encoding="utf-8") as f:
        pages = json.load(f)

    return headings, pages


def convert_to_markdown(book_info: dict, headings: list, pages: list) -> str:
    """Convert Turath headings + pages into a markdown string."""
    name_en = book_info["name_en"]

    # Build page lookup: page_id → text
    page_map = {}
    for p in pages:
        pid = p.get("page_id", p.get("id", 0))
        page_map[pid] = p.get("text", "")

    # Group headings by page
    heading_at_page: dict[int, list] = {}
    for h in headings:
        pg = h.get("page", 0)
        heading_at_page.setdefault(pg, []).append(h)

    # Build markdown
    lines = [f"# {name_en}\n"]

    for page_id in sorted(page_map.keys()):
        # Insert headings for this page
        if page_id in heading_at_page:
            for h in heading_at_page[page_id]:
                level = h.get("level", 1)
                # Map turath levels (1-5) to markdown (## to ######)
                # Reserve # for book title
                md_level = min(level + 1, 6)
                lines.append(f"{'#' * md_level} {h['title']}\n")

        # Add page text
        text = page_map[page_id].strip()
        if text:
            lines.append(text + "\n")

    return "\n".join(lines)


def build_tree(md_path: str, with_summaries: bool, model: str | None) -> dict:
    """Run PageIndex md_to_tree on a markdown file."""
    from pageindex.page_index_md import md_to_tree

    result = asyncio.run(
        md_to_tree(
            md_path=md_path,
            if_thinning=False,
            if_add_node_summary="yes" if with_summaries else "no",
            summary_token_threshold=200,
            model=model if with_summaries else None,
            if_add_doc_description="no",
            if_add_node_text="yes",
            if_add_node_id="yes",
        )
    )
    return result


def count_nodes(nodes: list) -> int:
    total = len(nodes)
    for n in nodes:
        total += count_nodes(n.get("nodes", []))
    return total


def build_book(book_info: dict, book_map: dict, with_summaries: bool, model: str | None):
    """Build PageIndex tree for a single book."""
    book_id = str(book_info["id"])
    slug = book_info["slug"]
    name_en = book_info["name_en"]

    print(f"\n{'='*60}")
    print(f"Building: {name_en} (book_id={book_id})")
    print(f"{'='*60}")

    # Check if already built
    tree_path = PAGEINDEX_DIR / f"{book_id}.json"
    if tree_path.exists() and book_id in book_map:
        print(f"  Already built, skipping (delete {tree_path.name} to rebuild)")
        return

    # Load data
    result = load_book_data(slug)
    if result is None:
        print(f"  SKIPPED: missing data files")
        return
    headings, pages = result
    print(f"  Headings: {len(headings)}, Pages: {len(pages)}")

    # Convert to markdown
    MARKDOWN_DIR.mkdir(parents=True, exist_ok=True)
    md_path = MARKDOWN_DIR / f"{slug}.md"
    print(f"  Converting to markdown...")
    md_content = convert_to_markdown(book_info, headings, pages)
    with open(md_path, "w", encoding="utf-8") as f:
        f.write(md_content)
    print(f"  Markdown: {len(md_content):,} chars, {md_content.count(chr(10)):,} lines")

    # Build tree
    print(f"  Building tree with PageIndex...")
    tree = build_tree(str(md_path), with_summaries, model)

    structure = tree.get("structure", [])
    total_nodes = count_nodes(structure)
    print(f"  Tree: {total_nodes} nodes, {tree.get('line_count', 0):,} lines")

    # Save tree JSON
    PAGEINDEX_DIR.mkdir(parents=True, exist_ok=True)
    with open(tree_path, "w", encoding="utf-8") as f:
        json.dump(tree, f, ensure_ascii=False, indent=2)

    # Update book map
    book_map[book_id] = {
        "name_en": name_en,
        "name_ar": book_info["name_ar"],
        "doc_id": book_id,
        "line_count": tree.get("line_count", 0),
        "node_count": total_nodes,
        "md_path": str(md_path),
    }
    save_book_map(book_map)
    print(f"  Saved: {tree_path.name}")


def load_book_map() -> dict:
    if BOOK_MAP_FILE.exists():
        with open(BOOK_MAP_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    return {}


def save_book_map(book_map: dict):
    PAGEINDEX_DIR.mkdir(parents=True, exist_ok=True)
    with open(BOOK_MAP_FILE, "w", encoding="utf-8") as f:
        json.dump(book_map, f, ensure_ascii=False, indent=2)


def show_status(book_map: dict):
    print("PageIndex Build Status:\n")
    for book in BOOKS:
        bid = str(book["id"])
        slug = book["slug"]
        has_headings = (DATA_DIR / f"{slug}_headings.json").exists()
        has_pages = (DATA_DIR / f"{slug}_pages.json").exists()
        built = bid in book_map

        data_status = "ready" if (has_headings and has_pages) else "missing data"
        if not has_headings:
            data_status = "no headings"
        elif not has_pages:
            data_status = "no pages"

        tree_status = f"built ({book_map[bid]['node_count']} nodes)" if built else "not built"
        print(f"  {bid}: {book['name_en']}")
        print(f"    Data: {data_status} | Tree: {tree_status}")
    print()


def main():
    book_map = load_book_map()

    if "--status" in sys.argv:
        show_status(book_map)
        return

    # Parse args
    with_summaries = "--with-summaries" in sys.argv
    model = os.environ.get("PAGEINDEX_MODEL", "ollama/command-r7b-arabic")
    book_filter = None

    args = sys.argv[1:]
    i = 0
    while i < len(args):
        if args[i] == "--model" and i + 1 < len(args):
            model = args[i + 1]
            i += 2
        elif args[i] == "--book-id" and i + 1 < len(args):
            book_filter = int(args[i + 1])
            i += 2
        else:
            i += 1

    books_to_build = BOOKS
    if book_filter:
        books_to_build = [b for b in BOOKS if b["id"] == book_filter]
        if not books_to_build:
            print(f"Error: Book ID {book_filter} not found.")
            sys.exit(1)

    if with_summaries:
        print(f"Model: {model} (for summaries)")
    else:
        print(f"No LLM needed (basic tree mode)")
    print(f"Books to build: {len(books_to_build)}")

    for book_info in books_to_build:
        build_book(book_info, book_map, with_summaries, model)

    print(f"\n{'='*60}")
    print(f"Done. Book map: {BOOK_MAP_FILE}")
    print(f"Books built: {len(book_map)}")


if __name__ == "__main__":
    main()
