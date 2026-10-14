import { describe, expect, it } from 'vitest';
import { isValidDueDate, normalizeTitle, todayLocalDateString } from '../src/domain/taskValidation';

describe('normalizeTitle', () => {
  it('trims whitespace from both ends', () => {
    expect(normalizeTitle('  buy milk  ')).toBe('buy milk');
    expect(normalizeTitle('\tзадача\n')).toBe('задача');
  });

  it('rejects empty and whitespace-only input', () => {
    expect(normalizeTitle('')).toBeNull();
    expect(normalizeTitle('   ')).toBeNull();
    expect(normalizeTitle('\t\n ')).toBeNull();
  });

  it('preserves unicode content', () => {
    expect(normalizeTitle('  Купить молоко 🥛  ')).toBe('Купить молоко 🥛');
  });
});

describe('isValidDueDate', () => {
  it('accepts real calendar dates', () => {
    expect(isValidDueDate('2026-10-13')).toBe(true);
    expect(isValidDueDate('2024-02-29')).toBe(true); // leap year
    expect(isValidDueDate('2000-02-29')).toBe(true); // divisible by 400
    expect(isValidDueDate('2099-12-31')).toBe(true);
  });

  it('rejects impossible dates', () => {
    expect(isValidDueDate('2025-02-29')).toBe(false); // not a leap year
    expect(isValidDueDate('2025-13-01')).toBe(false);
    expect(isValidDueDate('2025-00-10')).toBe(false);
    expect(isValidDueDate('2025-04-31')).toBe(false); // April has 30 days
  });

  it('rejects non-calendar formats', () => {
    expect(isValidDueDate('13-10-2026')).toBe(false);
    expect(isValidDueDate('2026-1-1')).toBe(false);
    expect(isValidDueDate('2026-10-13T10:00:00Z')).toBe(false);
    expect(isValidDueDate('завтра')).toBe(false);
    expect(isValidDueDate('')).toBe(false);
  });
});

describe('todayLocalDateString', () => {
  it('formats a local date as YYYY-MM-DD', () => {
    const date = new Date(2026, 9, 5, 15, 30); // 5 Oct 2026, 15:30 local
    expect(todayLocalDateString(date)).toBe('2026-10-05');
  });
});
