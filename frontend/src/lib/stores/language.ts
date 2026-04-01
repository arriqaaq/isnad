import { writable } from 'svelte/store';

export type Language = 'ar' | 'en';

const stored = typeof localStorage !== 'undefined'
  ? (localStorage.getItem('hadith-lang') as Language) || 'ar'
  : 'ar';

export const language = writable<Language>(stored);

language.subscribe(val => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('hadith-lang', val);
  }
});
