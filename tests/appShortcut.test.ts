import { describe, expect, it } from 'vitest';
import { appShortcut } from '../src/lib/appShortcut';

function event(overrides: Partial<KeyboardEvent> = {}) {
  return {
    key: 't',
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    ...overrides,
  } as unknown as KeyboardEvent;
}

describe('appShortcut', () => {
  it('reads the four bindings with either modifier', () => {
    // ⌘ on macOS, Ctrl on Windows and Linux: both accepted everywhere, so the
    // app needs no platform detection to be predictable.
    for (const modifier of ['metaKey', 'ctrlKey'] as const) {
      const meta = { [modifier]: true };
      expect(appShortcut(event({ ...meta, key: 't' }))).toBe('newTab');
      expect(appShortcut(event({ ...meta, key: 'f' }))).toBe('focusSearch');
      expect(appShortcut(event({ ...meta, key: 's' }))).toBe('settings');
      expect(appShortcut(event({ ...meta, key: 'd' }))).toBe('markNotRelevant');
    }
  });

  it('ignores the letters without a modifier', () => {
    // Typing "t" in the search field or a task title must never create a tab.
    expect(appShortcut(event({ key: 't' }))).toBeNull();
    expect(appShortcut(event({ key: 'f' }))).toBeNull();
    expect(appShortcut(event({ key: 's' }))).toBeNull();
    expect(appShortcut(event({ key: 'd' }))).toBeNull();
  });

  it('leaves Alt and Shift alone', () => {
    // ⌥⌘T and ⌘⇧F belong to whoever bound them.
    expect(appShortcut(event({ metaKey: true, altKey: true, key: 't' }))).toBeNull();
    expect(appShortcut(event({ metaKey: true, shiftKey: true, key: 'f' }))).toBeNull();
  });

  it('does not claim the digits or anything else', () => {
    // ⌘1..9 belongs to the tab strip; a key is one or the other.
    for (const key of ['1', '9', 'n', 'Enter', 'Escape', 'ArrowDown']) {
      expect(appShortcut(event({ metaKey: true, key }))).toBeNull();
    }
  });

  it('accepts a capital letter from a held Shift-free layout difference', () => {
    // With Caps Lock on, the key arrives as "T". Shift is still required to be
    // absent, so this does not open ⌘⇧T.
    expect(appShortcut(event({ metaKey: true, key: 'T' }))).toBe('newTab');
  });
});