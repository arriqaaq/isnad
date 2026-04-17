#!/usr/bin/env python3
"""
Download PDFs for all Turath books used by the hadith project.

Downloads volume PDFs via files.turath.io (proxies both archive.org and direct).
Resume-safe: skips files that already exist.

Usage:
  python3 scripts/download_turath_pdfs.py              # download all books
  python3 scripts/download_turath_pdfs.py --book 1673   # download one book
  python3 scripts/download_turath_pdfs.py --list        # list books and PDF info

Outputs:
  data/pdfs/{book_id}/vol_{N}.pdf  — volume PDFs
  data/pdfs/manifest.json          — book metadata + volume mapping
"""

import json
import os
import sys
import time
import urllib.parse
import urllib.request

API_BASE = "https://api.turath.io"
PDF_BASE = "https://files.turath.io/pdf"

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(os.path.dirname(SCRIPT_DIR), "data")
PDF_DIR = os.path.join(DATA_DIR, "pdfs")
MANIFEST_FILE = os.path.join(PDF_DIR, "manifest.json")

# All 8 Turath books used in the project
BOOKS = [
    {"id": 1673, "name_en": "Fath al-Bari", "name_ar": "فتح الباري بشرح البخاري"},
    {"id": 23604, "name_en": "Tafsir Ibn Kathir", "name_ar": "تفسير ابن كثير"},
    {"id": 1711, "name_en": "Sharh Nawawi ala Muslim", "name_ar": "شرح النووي على مسلم"},
    {"id": 21662, "name_en": "Tuhfat al-Ahwadhi", "name_ar": "تحفة الأحوذي"},
    {"id": 1147, "name_en": "Sahih Sunan al-Nasai", "name_ar": "صحيح سنن النسائي"},
    {"id": 5760, "name_en": "Awn al-Mabud", "name_ar": "عون المعبود"},
    {"id": 98138, "name_en": "Sunan Ibn Majah", "name_ar": "سنن ابن ماجه"},
    {"id": 1278, "name_en": "Tahdhib al-Tahdhib", "name_ar": "تهذيب التهذيب"},
]


def fetch_json(url: str, retries: int = 3) -> dict:
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "hadith-project/1.0")
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except Exception as e:
            if attempt < retries - 1:
                wait = 2 ** (attempt + 1)
                print(f"  Retry {attempt+1} after error: {e} (waiting {wait}s)")
                time.sleep(wait)
            else:
                raise


def prepare_pdf_url(root: str, filename: str) -> str:
    """Construct download URL using usul.ai's pattern."""
    # Strip the label suffix (e.g., "shsm00.pdf|الغلاف" → "shsm00.pdf")
    clean_file = filename.split("|")[0]

    # Combine root + filename
    if root:
        final = root.rstrip("/") + "/" + clean_file
    else:
        final = clean_file

    # If it's an archive.org URL, rewrite to files.turath.io proxy format
    if "archive.org" in final:
        # "https://archive.org/download/ftbsbkhslfiya/01_2022.pdf"
        #  → "archive/ftbsbkhslfiya_=_01_2022.pdf"
        path = final.replace("https://archive.org/download/", "")
        # Replace FIRST slash only with _=_
        path = path.replace("/", "_=_", 1)
        return f"{PDF_BASE}/archive/{path}"
    else:
        # Relative path — URL-encode the Arabic characters
        encoded = urllib.parse.quote(final)
        return f"{PDF_BASE}/{encoded}"


def get_label(filename: str) -> str:
    """Extract label from filename (e.g., 'shsm00.pdf|الغلاف' → 'الغلاف')."""
    if "|" in filename:
        return filename.split("|", 1)[1]
    return ""


def download_file(url: str, dest: str, retries: int = 3) -> bool:
    """Download a file with retry and resume support."""
    if os.path.exists(dest):
        return True  # already downloaded

    for attempt in range(retries):
        try:
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "hadith-project/1.0")
            with urllib.request.urlopen(req, timeout=120) as resp:
                total = int(resp.headers.get("Content-Length", 0))
                data = resp.read()
                # Write to temp file first, then rename (atomic)
                tmp = dest + ".tmp"
                with open(tmp, "wb") as f:
                    f.write(data)
                os.rename(tmp, dest)
                size_mb = len(data) / (1024 * 1024)
                print(f"    Downloaded {size_mb:.1f} MB")
                return True
        except Exception as e:
            if attempt < retries - 1:
                wait = 2 ** (attempt + 1)
                print(f"    Retry {attempt+1}: {e} (waiting {wait}s)")
                time.sleep(wait)
            else:
                print(f"    FAILED: {e}")
                # Clean up partial file
                for f in [dest, dest + ".tmp"]:
                    if os.path.exists(f):
                        os.remove(f)
                return False


def fetch_book_pdfs(book_info: dict) -> dict | None:
    """Fetch PDF metadata and download all volumes for a book."""
    book_id = book_info["id"]
    name_en = book_info["name_en"]

    print(f"\n{'='*60}")
    print(f"Book {book_id}: {name_en}")
    print(f"{'='*60}")

    # Fetch book metadata
    url = f"{API_BASE}/book?id={book_id}&ver=3"
    print(f"  Fetching metadata...")
    data = fetch_json(url)
    meta = data.get("meta", {})
    pdf_links = meta.get("pdf_links", {})

    if not pdf_links or not pdf_links.get("files"):
        print(f"  WARNING: No PDF links found for {name_en}")
        return None

    root = pdf_links.get("root", "")
    files = pdf_links["files"]
    print(f"  Root: {root}")
    print(f"  Volumes: {len(files)}")

    # Create book directory
    book_dir = os.path.join(PDF_DIR, str(book_id))
    os.makedirs(book_dir, exist_ok=True)

    # Download each volume
    volumes = []
    for i, filename in enumerate(files):
        label = get_label(filename)
        vol_num = i  # 0-indexed (vol 0 is often the cover/intro)
        dest = os.path.join(book_dir, f"vol_{vol_num:02d}.pdf")
        dl_url = prepare_pdf_url(root, filename)

        label_str = f" ({label})" if label else ""
        print(f"  [{vol_num:02d}/{len(files)-1}] {filename.split('|')[0]}{label_str}")

        if os.path.exists(dest):
            print(f"    Already exists, skipping")
            success = True
        else:
            print(f"    URL: {dl_url}")
            success = download_file(dl_url, dest)
            time.sleep(0.3)  # rate limit

        if success:
            volumes.append({
                "vol": vol_num,
                "filename": filename.split("|")[0],
                "label": label,
                "path": os.path.relpath(dest, DATA_DIR),
            })

    print(f"  Done: {len(volumes)}/{len(files)} volumes downloaded")

    return {
        "book_id": book_id,
        "name_en": name_en,
        "name_ar": book_info["name_ar"],
        "api_name": meta.get("name", ""),
        "root": root,
        "volumes": volumes,
    }


def list_books():
    """List all books with their PDF info (no download)."""
    for book in BOOKS:
        book_id = book["id"]
        url = f"{API_BASE}/book?id={book_id}&ver=3"
        try:
            data = fetch_json(url)
            meta = data.get("meta", {})
            pdf_links = meta.get("pdf_links", {})
            files = pdf_links.get("files", [])
            root = pdf_links.get("root", "")
            print(f"  {book_id}: {book['name_en']}")
            print(f"    API name: {meta.get('name', '')}")
            print(f"    Root: {root}")
            print(f"    Volumes: {len(files)}")
            # Check if already downloaded
            book_dir = os.path.join(PDF_DIR, str(book_id))
            if os.path.exists(book_dir):
                existing = len([f for f in os.listdir(book_dir) if f.endswith(".pdf")])
                print(f"    Downloaded: {existing}/{len(files)}")
            print()
        except Exception as e:
            print(f"  {book_id}: {book['name_en']} — ERROR: {e}")
            print()


def main():
    os.makedirs(PDF_DIR, exist_ok=True)

    if "--list" in sys.argv:
        print("Turath Books PDF Status:\n")
        list_books()
        return

    # Filter to single book if --book specified
    book_filter = None
    if "--book" in sys.argv:
        idx = sys.argv.index("--book")
        if idx + 1 < len(sys.argv):
            book_filter = int(sys.argv[idx + 1])

    books_to_fetch = BOOKS
    if book_filter:
        books_to_fetch = [b for b in BOOKS if b["id"] == book_filter]
        if not books_to_fetch:
            print(f"Error: Book ID {book_filter} not found. Available: {[b['id'] for b in BOOKS]}")
            sys.exit(1)

    # Load existing manifest
    manifest = {}
    if os.path.exists(MANIFEST_FILE):
        with open(MANIFEST_FILE, "r", encoding="utf-8") as f:
            manifest = json.load(f)

    # Download each book
    for book_info in books_to_fetch:
        result = fetch_book_pdfs(book_info)
        if result:
            manifest[str(result["book_id"])] = result
            # Save manifest after each book (resume support)
            with open(MANIFEST_FILE, "w", encoding="utf-8") as f:
                json.dump(manifest, f, ensure_ascii=False, indent=2)

    print(f"\n{'='*60}")
    print(f"Manifest saved to {MANIFEST_FILE}")
    total_vols = sum(len(m["volumes"]) for m in manifest.values())
    print(f"Total: {len(manifest)} books, {total_vols} volumes")


if __name__ == "__main__":
    main()
