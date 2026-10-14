import { describe, expect, it } from 'vitest';
import { fill } from '../src/lib/text';

describe('fill', () => {
  it('substitutes a placeholder', () => {
    expect(fill('Задача «{title}»', 'title', 'Отчёт')).toBe('Задача «Отчёт»');
  });

  it('leaves unknown placeholders untouched', () => {
    expect(fill('Задача «{title}»', 'other', 'x')).toBe('Задача «{title}»');
  });

  // `$&`, `$'` and `` $` `` are replacement patterns in String.replace; a task
  // name containing them must survive verbatim.
  it('does not expand dollar sequences in the value', () => {
    expect(fill('Задача «{title}»', 'title', 'Купить $& хлеб')).toBe(
      'Задача «Купить $& хлеб»',
    );
    expect(fill('Задача «{title}»', 'title', "Цена $' руб")).toBe(
      'Задача «Цена $\' руб»',
    );
    expect(fill('Задача «{title}»', 'title', 'A $` B')).toBe('Задача «A $` B»');
  });

  it('handles empty values', () => {
    expect(fill('Задача «{title}»', 'title', '')).toBe('Задача «»');
  });
});