import { describe, expect, it } from 'vitest';
import { onEnter, onEscape, onLeave } from '../src/lib/capture';

/**
 * The storm: type, Enter, type, Enter. These rules are shared by the root
 * input and the inline subtask field, so the two must never drift apart.
 */
describe('Enter in a quick-entry field', () => {
  it('saves the trimmed title', () => {
    expect(onEnter('  Купить молоко  ')).toEqual({ kind: 'add', title: 'Купить молоко' });
  });

  it('does nothing on an empty or blank field', () => {
    expect(onEnter('')).toEqual({ kind: 'idle' });
    expect(onEnter('    ')).toEqual({ kind: 'idle' });
  });

  it('ignores a repeated Enter while a save is running', () => {
    // Without this a fast typist creates the same task twice.
    expect(onEnter('Задача', true)).toEqual({ kind: 'idle' });
  });

  it('can be used any number of times in a row', () => {
    // Each call is independent: the field never latches.
    expect(onEnter('Первая')).toEqual({ kind: 'add', title: 'Первая' });
    expect(onEnter('Вторая')).toEqual({ kind: 'add', title: 'Вторая' });
    expect(onEnter('Третья')).toEqual({ kind: 'add', title: 'Третья' });
  });
});

describe('Escape in a quick-entry field', () => {
  it('cancels: nothing typed is created', () => {
    expect(onEscape()).toEqual({ kind: 'cancel' });
  });
});

describe('leaving the field with the pointer', () => {
  it('saves what is typed instead of losing it', () => {
    expect(onLeave('Незаконченная мысль')).toEqual({
      kind: 'save-and-close',
      title: 'Незаконченная мысль',
    });
  });

  it('closes an empty field without touching the database', () => {
    expect(onLeave('')).toEqual({ kind: 'cancel' });
    expect(onLeave('  ')).toEqual({ kind: 'cancel' });
  });

  it('never leaves the field stuck open', () => {
    // Even when a save is already running the field must close, otherwise a
    // stray click during a storm would leave a ghost input behind.
    expect(onLeave('Задача', true)).toEqual({ kind: 'cancel' });
  });
});