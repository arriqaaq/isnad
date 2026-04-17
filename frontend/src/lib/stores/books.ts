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

export function getTafsirBookId(config: BooksConfig): number | null {
  return config.tafsir_book_id;
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
