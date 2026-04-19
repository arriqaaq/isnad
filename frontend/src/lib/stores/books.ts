import { getBooksConfig } from '$lib/api';
import type { BooksConfig, BookConfig } from '$lib/types';

let cached: BooksConfig | null = null;
let loading: Promise<BooksConfig> | null = null;

export async function loadBooksConfig(): Promise<BooksConfig> {
  if (cached) return cached;
  if (loading) return loading;

  loading = getBooksConfig().then((config) => {
    cached = config;
    loading = null;
    return config;
  });

  return loading;
}

/**
 * Returns the default tafsir's `book_id`, or null if no tafsirs are configured.
 * Defaults are marked server-side (currently Ibn Kathir); falls back to the
 * first entry if no explicit default is flagged.
 */
export function getTafsirBookId(config: BooksConfig): number | null {
  const list = config.tafsir_books ?? [];
  if (list.length === 0) return null;
  const dflt = list.find((t) => t.is_default);
  return (dflt ?? list[0]).book_id;
}

export function getBookConfig(config: BooksConfig, bookId: number): BookConfig | undefined {
  return config.books.find((b) => b.book_id === bookId);
}

export function getBookName(config: BooksConfig, bookId: number): string {
  const book = getBookConfig(config, bookId);
  return book?.name_en ?? 'Unknown Book';
}

export function getBookNameAr(config: BooksConfig, bookId: number): string {
  const book = getBookConfig(config, bookId);
  return book?.name_ar ?? '';
}
