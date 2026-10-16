import { describe, expect, it } from 'vitest';
import { isTabShortcut, tabForDigit } from '../src/lib/tabShortcut';

// Tabs were reachable only by clicking. Adding ⌘1..9 is the convention every
// app with tabs already follows, and the part that is easy to get subtly wrong
// is which digit selects what — so that is what these pin down.

const keys = (over: Partial<Parameters<typeof isTabShortcut>[0]> = {}) => ({
  key: '1',
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...over,
});

const tabs = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];

describe('tab shortcut keys', () => {
  it('takes Cmd on macOS and Ctrl elsewhere', () => {
    expect(isTabShortcut(keys({ metaKey: true }))).toBe(true);
    expect(isTabShortcut(keys({ ctrlKey: true }))).toBe(true);
  });

  it('ignores a digit without a modifier', () => {
    // Typing "1" into the task input must not switch anything.
    expect(isTabShortcut(keys())).toBe(false);
  });

  it('leaves combinations with Alt or Shift to whoever bound them', () => {
    expect(isTabShortcut(keys({ metaKey: true, shiftKey: true }))).toBe(false);
    expect(isTabShortcut(keys({ ctrlKey: true, altKey: true }))).toBe(false);
  });

  it('ignores zero, more than nine, and anything that is not a plain digit', () => {
    for (const key of ['0', '10', 'a', 'Enter', ' ', '-', 'F1']) {
      expect(isTabShortcut(keys({ metaKey: true, key })), key).toBe(false);
    }
  });
});

describe('which tab a digit selects', () => {
  it('counts from the left, in strip order', () => {
    expect(tabForDigit(tabs, 1)).toBe('a');
    expect(tabForDigit(tabs, 2)).toBe('b');
    expect(tabForDigit(tabs, 3)).toBe('c');
  });

  it('follows the order the strip shows, not the order tabs were created', () => {
    // Tabs are dragged around, so the number has to track what is on screen.
    const reordered = [{ id: 'c' }, { id: 'a' }, { id: 'b' }];
    expect(tabForDigit(reordered, 1)).toBe('c');
    expect(tabForDigit(reordered, 2)).toBe('a');
  });

  it('selects nothing past the last tab instead of wrapping', () => {
    // Wrapping would make ⌘7 land somewhere unpredictable every time a tab is
    // closed.
    expect(tabForDigit(tabs, 4)).toBeNull();
    expect(tabForDigit([], 1)).toBeNull();
  });

  it('rejects a digit that is not in range', () => {
    expect(tabForDigit(tabs, 0)).toBeNull();
    expect(tabForDigit(tabs, -1)).toBeNull();
    expect(tabForDigit(tabs, 1.5)).toBeNull();
  });
});