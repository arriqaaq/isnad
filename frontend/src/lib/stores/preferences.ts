import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark' | 'brown' | 'pink';
export type QuranFontMode = 'uthmani' | 'madani' | 'tajweed';

export interface QuranPreferences {
  arabicFontSize: number;
  englishFontSize: number;
  theme: Theme;
  selectedReciter: string | null;
  quranFont: QuranFontMode;
}

export const FONT_STEPS = [0.6, 0.8, 0.9, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 2.4, 2.8];

export const DEFAULTS: QuranPreferences = {
  arabicFontSize: 1.6,
  englishFontSize: 0.9,
  theme: 'light',
  selectedReciter: null,
  quranFont: 'uthmani',
};

const STORAGE_KEY = 'quran-preferences';

function load(): QuranPreferences {
  if (typeof localStorage === 'undefined') return DEFAULTS;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? { ...DEFAULTS, ...JSON.parse(raw) } : DEFAULTS;
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
