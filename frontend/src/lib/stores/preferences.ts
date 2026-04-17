import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark' | 'brown';
export type QuranFontMode = 'uthmani' | 'madani' | 'tajweed';

export interface QuranPreferences {
  arabicFontSize: number;
  englishFontSize: number;
  theme: Theme;
  selectedReciter: string | null;
  quranFont: QuranFontMode;
  sidebarCollapsed: boolean;
}

export const FONT_STEPS = [0.6, 0.8, 0.9, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 2.4, 2.8, 3.2, 3.6];

export const DEFAULTS: QuranPreferences = {
  arabicFontSize: 2.4,
  englishFontSize: 1.0,
  theme: 'light',
  selectedReciter: null,
  quranFont: 'tajweed',
  sidebarCollapsed: false,
};

const STORAGE_KEY = 'quran-preferences';

function load(): QuranPreferences {
  if (typeof localStorage === 'undefined') return DEFAULTS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULTS;
    const saved = { ...DEFAULTS, ...JSON.parse(raw) };
    // Migrate: uthmani font has broken glyph coverage, switch to tajweed
    if (saved.quranFont === 'uthmani') {
      saved.quranFont = 'tajweed';
    }
    // Migrate: pink theme removed
    if ((saved.theme as string) === 'pink') {
      saved.theme = 'light';
    }
    return saved;
  } catch {
    return DEFAULTS;
  }
}

export const preferences = writable<QuranPreferences>(load());

preferences.subscribe((v) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(v));
  }
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = v.theme;
  }
});

export function stepSize(current: number, direction: 1 | -1): number {
  const idx = FONT_STEPS.indexOf(current);
  if (idx === -1) {
    const nearest = FONT_STEPS.reduce((a, b) =>
      Math.abs(b - current) < Math.abs(a - current) ? b : a
    );
    const nearIdx = FONT_STEPS.indexOf(nearest);
    const next = nearIdx + direction;
    return FONT_STEPS[Math.max(0, Math.min(next, FONT_STEPS.length - 1))];
  }
  const next = idx + direction;
  return FONT_STEPS[Math.max(0, Math.min(next, FONT_STEPS.length - 1))];
}
