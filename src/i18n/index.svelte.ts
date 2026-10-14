/**
 * Minimal reactive i18n.
 *
 * The active language is kept in a `$state` field (same pattern as the stores,
 * which `svelte-check` understands) inside a `.svelte.ts` module, so every
 * component calling `t()` in its markup re-renders when the language changes.
 */
export type Language = 'ru' | 'en';

import { ru } from './ru';
import { en } from './en';

const dictionaries: Record<Language, Record<string, string>> = { ru, en };

class I18n {
  current = $state<Language>('ru');

  set(lang: Language): void {
    this.current = lang;
    if (typeof document !== 'undefined') document.documentElement.lang = lang;
  }

  get(): Language {
    return this.current;
  }

  /** Translates a dot-path; falls back to Russian, then to the key itself. */
  translate(path: string, vars?: Record<string, string>): string {
    const template = dictionaries[this.current][path] ?? dictionaries.ru[path] ?? path;
    if (!vars) return template;
    // Functional replacement: a value containing `$&` must stay literal.
    return Object.entries(vars).reduce(
      (text, [key, value]) => text.replace(`{${key}}`, () => value),
      template,
    );
  }
}

const i18n = new I18n();

export function setLanguage(lang: Language): void {
  i18n.set(lang);
}

export function getLanguage(): Language {
  return i18n.get();
}

export function t(path: string, vars?: Record<string, string>): string {
  return i18n.translate(path, vars);
}