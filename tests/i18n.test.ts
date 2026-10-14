import { describe, expect, it } from 'vitest';
import { ru } from '../src/i18n/ru';
import { en } from '../src/i18n/en';
import { setLanguage, t } from '../src/i18n/index.svelte';

describe('i18n', () => {
  it('switches language at runtime', () => {
    setLanguage('ru');
    expect(t('settings.title')).toBe('Настройки');
    setLanguage('en');
    expect(t('settings.title')).toBe('Settings');
    setLanguage('ru');
  });

  it('reports changes to reactive readers', () => {
    // Regression guard: the language used to live in a plain module variable,
    // so components kept rendering the old language until a reload.
    setLanguage('ru');
    const before = t('settings.deleteForever');
    setLanguage('en');
    const after = t('settings.deleteForever');
    expect(before).toBe('Удалить');
    expect(after).toBe('Delete');
    setLanguage('ru');
  });

  it('translates every key used by the UI', () => {
    const missingInEn: string[] = [];
    for (const key of Object.keys(ru)) {
      if (!en[key]) missingInEn.push(key);
    }
    expect(missingInEn).toEqual([]);
  });

  it('has no stale English-only keys', () => {
    const extraInEn = Object.keys(en).filter((key) => !ru[key]);
    expect(extraInEn).toEqual([]);
  });

  it('keeps the same placeholders in both languages', () => {
    const placeholders = (value: string) => (value.match(/\{[a-zA-Z]+\}/g) ?? []).sort();
    const mismatched: string[] = [];
    for (const key of Object.keys(ru)) {
      if (placeholders(ru[key]).join(',') !== placeholders(en[key]).join(',')) {
        mismatched.push(key);
      }
    }
    expect(mismatched).toEqual([]);
  });

  it('falls back to the key when a translation is missing', () => {
    setLanguage('en');
    expect(t('nope.not.here')).toBe('nope.not.here');
    setLanguage('ru');
  });
});