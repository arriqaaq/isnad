import { getTurathBooksConfig } from '$lib/api';
import type { TurathBooksConfig, TurathBookConfig } from '$lib/types';

let cached: TurathBooksConfig | null = null;
let loading: Promise<TurathBooksConfig> | null = null;

export async function loadTurathConfig(): Promise<TurathBooksConfig> {
  if (cached) return cached;
  if (loading) return loading;

  loading = getTurathBooksConfig().then((config) => {
    cached = config;
    loading = null;
    return config;
  });

  return loading;
}

export function getTafsirBookId(config: TurathBooksConfig): number | null {
  return config.tafsir_book_id;
}

export function getBookConfig(config: TurathBooksConfig, bookId: number): TurathBookConfig | undefined {
  return config.books.find((b) => b.book_id === bookId);
}

export function getBookName(config: TurathBooksConfig, bookId: number): string {
  const book = getBookConfig(config, bookId);
  return book?.name_en ?? 'Unknown Book';
}

export function getBookNameAr(config: TurathBooksConfig, bookId: number): string {
  const book = getBookConfig(config, bookId);
  return book?.name_ar ?? '';
}
