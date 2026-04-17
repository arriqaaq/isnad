"""Shared fetch helpers for turath.io book downloads.

Used by every scripts/fetch_*.py. Owns:
  - fetch_json: single GET with retry/backoff (stdlib urllib only).
  - fetch_book_metadata: one-shot /book?include=indexes call, written to disk.
  - fetch_all_pages: concurrent /page fetches via a bounded ThreadPoolExecutor,
    with resume support and atomic checkpoint writes.

The output JSON shape is byte-for-byte the same as the previous per-script
implementations: a list of {"page_id", "meta", "text"} sorted by page_id.

Politeness knobs:
  - workers defaults to 8; override via TURATH_WORKERS env var.
  - Retries are exponential (2s, 4s, 8s) inside fetch_json.
  - No per-request sleep; the bounded executor is the throttle.
"""

import json
import os
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed

API_BASE = "https://api.turath.io"
USER_AGENT = "hadith-project/1.0"


def default_workers(default: int = 8) -> int:
    raw = os.environ.get("TURATH_WORKERS", "").strip()
    if not raw:
        return default
    try:
        n = int(raw)
        return n if n > 0 else default
    except ValueError:
        return default


def fetch_json(url: str, retries: int = 3) -> dict:
    last_exc = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url)
            req.add_header("User-Agent", USER_AGENT)
            with urllib.request.urlopen(req, timeout=30) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except Exception as e:
            last_exc = e
            if attempt < retries - 1:
                wait = 2 ** (attempt + 1)
                print(f"  Retry {attempt+1} after error: {e} (waiting {wait}s)")
                time.sleep(wait)
    raise last_exc  # type: ignore[misc]


def fetch_book_metadata(book_id: int, display_name: str, headings_path: str) -> dict:
    if os.path.exists(headings_path):
        print(f"Headings already exist at {headings_path}, loading...")
        with open(headings_path, "r", encoding="utf-8") as f:
            return json.load(f)

    print(f"Fetching book metadata for {display_name} (book_id={book_id})...")
    url = f"{API_BASE}/book?id={book_id}&include=indexes&ver=3"
    data = fetch_json(url)

    os.makedirs(os.path.dirname(headings_path), exist_ok=True)
    _atomic_write_json(headings_path, data, indent=2)

    meta = data.get("meta", {})
    indexes = data.get("indexes", {})
    headings = indexes.get("headings", [])
    page_map = indexes.get("page_map", data.get("page_map", []))
    print(f"  Book: {meta.get('name', 'unknown')}")
    print(f"  Headings count: {len(headings)}")
    print(f"  Page map entries: {len(page_map)}")
    print(f"  Volumes: {indexes.get('volumes', [])}")
    print(f"  Saved to {headings_path}")
    return data


def _atomic_write_json(path: str, obj, *, indent=None) -> None:
    """Write JSON atomically: tmp file + os.replace.

    Prevents corruption if the process is killed mid-write.
    """
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        if indent is not None:
            json.dump(obj, f, ensure_ascii=False, indent=indent)
        else:
            json.dump(obj, f, ensure_ascii=False)
    os.replace(tmp, path)


def _fetch_page(book_id: int, page_id: int):
    """Fetch a single page. Returns the normalized record, or None for empty responses."""
    url = f"{API_BASE}/page?book_id={book_id}&pg={page_id}&ver=3"
    data = fetch_json(url)
    if not data or (not data.get("text") and not data.get("meta")):
        return None
    meta_raw = data.get("meta", {})
    if isinstance(meta_raw, str):
        try:
            meta_raw = json.loads(meta_raw)
        except json.JSONDecodeError:
            meta_raw = {}
    return {
        "page_id": page_id,
        "meta": meta_raw,
        "text": data.get("text", ""),
    }


def fetch_all_pages(
    metadata: dict,
    book_id: int,
    pages_path: str,
    *,
    workers: int = 8,
    checkpoint_every: int = 200,
    fallback_total: int = 5000,
    display_name: str = "",
) -> list:
    """Fetch every page of the book using a bounded thread pool.

    Resumes from existing pages_path if present. Pages may complete out of
    order; output JSON is always sorted by page_id.
    """
    indexes_obj = metadata.get("indexes", {})
    page_map = indexes_obj.get("page_map", metadata.get("page_map", []))
    total = len(page_map) or fallback_total

    suffix = f" for {display_name}" if display_name else ""
    print(f"Fetching all {total} pages{suffix} with {workers} workers...")

    pages_by_id: dict = {}
    if os.path.exists(pages_path):
        try:
            with open(pages_path, "r", encoding="utf-8") as f:
                existing = json.load(f)
            for p in existing:
                pid = p.get("page_id")
                if isinstance(pid, int):
                    pages_by_id[pid] = p
            if pages_by_id:
                print(f"  Loaded {len(pages_by_id)} previously fetched pages from {pages_path}")
        except (json.JSONDecodeError, KeyError, OSError):
            pages_by_id = {}

    missing = [pid for pid in range(1, total + 1) if pid not in pages_by_id]
    if not missing:
        print(f"  Nothing to fetch; {len(pages_by_id)} pages already on disk.")
        _atomic_write_json(pages_path, _sorted_pages(pages_by_id))
        return _sorted_pages(pages_by_id)

    print(f"  {len(missing)} pages missing; starting concurrent fetch...")

    completed_since_checkpoint = 0
    completed_total = 0
    empty_count = 0
    error_count = 0

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(_fetch_page, book_id, pid): pid for pid in missing}
        for fut in as_completed(futures):
            pid = futures[fut]
            try:
                rec = fut.result()
            except Exception as e:
                error_count += 1
                print(f"  Error fetching page {pid} after retries: {e}")
                continue

            completed_total += 1
            completed_since_checkpoint += 1

            if rec is None:
                empty_count += 1
            else:
                pages_by_id[pid] = rec

            if completed_total % checkpoint_every == 0:
                print(
                    f"  Fetched {completed_total} / {len(missing)} pages "
                    f"(have {len(pages_by_id)} total, latest pid={pid})..."
                )
                _atomic_write_json(pages_path, _sorted_pages(pages_by_id))
                completed_since_checkpoint = 0

    _atomic_write_json(pages_path, _sorted_pages(pages_by_id))

    print(f"  Total pages on disk: {len(pages_by_id)}")
    if empty_count:
        print(f"  Empty responses skipped: {empty_count}")
    if error_count:
        print(f"  Failed page fetches: {error_count} (re-run to retry)")
    print(f"  Saved to {pages_path}")

    return _sorted_pages(pages_by_id)


def _sorted_pages(pages_by_id: dict) -> list:
    return [pages_by_id[k] for k in sorted(pages_by_id.keys())]


def run_book(
    *,
    book_id: int,
    display_name: str,
    headings_path: str,
    pages_path: str,
    pages_required_flag: str = "--pages",
    skip_message: str = "",
) -> dict:
    """Convenience entry point used by per-book stubs.

    Mirrors the old `main()` shape: always fetch metadata; only fetch pages
    when sys.argv contains pages_required_flag.
    """
    os.makedirs(os.path.dirname(headings_path), exist_ok=True)
    metadata = fetch_book_metadata(book_id, display_name, headings_path)

    if pages_required_flag in sys.argv:
        fetch_all_pages(
            metadata,
            book_id,
            pages_path,
            workers=default_workers(),
            display_name=display_name,
        )
    else:
        indexes = metadata.get("indexes", {})
        page_map = indexes.get("page_map", [])
        msg = skip_message or (
            f"Use {pages_required_flag} to fetch all {len(page_map)} pages."
        )
        print(f"\nSkipping page fetch. {msg}")

    return metadata
