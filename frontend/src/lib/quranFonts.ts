/**
 * Quran.com-style glyph-based font system.
 *
 * Three rendering modes:
 *   - 'uthmani': Plain Unicode Arabic with UthmanicHafs (default, no external calls)
 *   - 'madani':  QCF V2 glyph codes with per-page black font files
 *   - 'tajweed': QCF V2 glyph codes with per-page colored V4 font files (COLRv1)
 *
 * V4 tajweed uses the SAME glyph codepoints as V2 — only the font files differ.
 * Font files are loaded on demand per mushaf page (~30-120KB each).
 */

export type QuranFontMode = 'uthmani' | 'madani' | 'tajweed';

interface GlyphVerse {
  code_v2: string;
  page: number;
}

// Cache: chapter number → map of verse_key → glyph data
const glyphCache = new Map<number, Map<string, GlyphVerse>>();

// Cache: loaded font pages (keyed by "v2:39" or "v4:39")
const loadedFonts = new Set<string>();

/**
 * Fetch glyph data for an entire chapter from quran.com API.
 * Cached per session — only fetches once per chapter.
 */
export async function fetchGlyphData(chapter: number): Promise<Map<string, GlyphVerse>> {
  if (glyphCache.has(chapter)) {
    return glyphCache.get(chapter)!;
  }

  const resp = await fetch(
    `https://api.quran.com/api/v4/quran/verses/code_v2?chapter_number=${chapter}`
  );
  if (!resp.ok) throw new Error(`Failed to fetch glyph data for chapter ${chapter}`);

  const data = await resp.json();
  const map = new Map<string, GlyphVerse>();

  for (const verse of data.verses) {
    map.set(verse.verse_key, {
      code_v2: verse.code_v2,
      page: verse.v2_page,
    });
  }

  glyphCache.set(chapter, map);
  return map;
}

/**
 * Load a per-page QCF font file into the document.
 * V2 = black glyphs, V4 = colored tajweed glyphs.
 */
export async function loadPageFont(page: number, mode: 'madani' | 'tajweed'): Promise<void> {
  const variant = mode === 'tajweed' ? 'v4' : 'v2';
  const key = `${variant}:${page}`;

  if (loadedFonts.has(key)) return;

  const padded = String(page).padStart(1, '0');
  const fontName = `QCF2_P${padded}`;
  const url = `/fonts/quran/${variant}/p${page}.woff2`;

  const face = new FontFace(fontName, `url('${url}') format('woff2')`);
  await face.load();
  document.fonts.add(face);
  loadedFonts.add(key);
}

/**
 * Get the CSS font-family for a specific mushaf page.
 * Falls back to sans-serif to prevent FOUT with wrong glyphs.
 */
export function getPageFontFamily(page: number): string {
  const padded = String(page).padStart(1, '0');
  return `'QCF2_P${padded}'`;
}

/**
 * Get glyph text and page for a specific verse.
 * Returns null if glyph data not yet loaded for this chapter.
 */
export function getVerseGlyph(chapter: number, verse: number): GlyphVerse | null {
  const map = glyphCache.get(chapter);
  if (!map) return null;
  return map.get(`${chapter}:${verse}`) ?? null;
}
