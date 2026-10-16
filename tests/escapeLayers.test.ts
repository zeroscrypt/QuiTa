import { describe, expect, it } from 'vitest';
import { UiStore } from '../src/stores/uiStore.svelte';

// Escape used to be handled by each overlay on its own document listener, so
// one press closed everything open at once: a date picker, the comment behind
// it and the menu that opened them. The stack is what makes one press close
// exactly one layer, innermost first.

function stack() {
  const store = new UiStore();
  const closed: string[] = [];
  return { store, closed };
}

describe('escape layers', () => {
  it('closes nothing when there is no layer', () => {
    const { store } = stack();
    expect(store.dismissTopmost()).toBe(false);
  });

  it('closes one layer per press, innermost first', () => {
    const { store, closed } = stack();
    // Opened the way the interface does: menu, then the popover it opened.
    store.openLayer('context-menu', () => closed.push('context-menu'));
    store.openLayer('due-date', () => closed.push('due-date'));
    store.openLayer('comment', () => closed.push('comment'));

    expect(store.dismissTopmost()).toBe(true);
    expect(store.dismissTopmost()).toBe(true);
    expect(store.dismissTopmost()).toBe(true);

    expect(closed).toEqual(['comment', 'due-date', 'context-menu']);
  });

  it('ignores layers that already closed themselves', () => {
    const { store, closed } = stack();
    store.openLayer('context-menu', () => closed.push('context-menu'));
    const leave = store.openLayer('due-date', () => closed.push('due-date'));

    // Clicking away closes the popover without going through Escape.
    leave();
    expect(store.layerCount).toBe(1);

    expect(store.dismissTopmost()).toBe(true);
    expect(closed).toEqual(['context-menu']);
  });

  it('does not let a stale layer swallow a later one of the same name', () => {
    const { store, closed } = stack();
    store.openLayer('due-date', () => closed.push('older'));
    const leave = store.openLayer('due-date', () => closed.push('newer'));

    // Reopening before the older one tore down must not unregister both.
    expect(store.layerCount).toBe(2);
    leave();
    expect(store.layerCount).toBe(1);
    expect(store.dismissTopmost()).toBe(true);
    expect(closed).toEqual(['older']);
  });

  it('drops the layer before closing it, so one press never repeats', () => {
    const { store, closed } = stack();
    // A layer whose close does not unmount it: without removing the entry
    // first, the next press would hit the same layer again.
    store.openLayer('stuck', () => closed.push('stuck'));

    expect(store.dismissTopmost()).toBe(true);
    expect(store.layerCount).toBe(0);
    expect(store.dismissTopmost()).toBe(false);
    expect(closed).toEqual(['stuck']);
  });

  it('stops once the stack is empty instead of repeating the last layer', () => {
    const { store, closed } = stack();
    store.openLayer('settings', () => closed.push('settings'));

    expect(store.dismissTopmost()).toBe(true);
    expect(store.dismissTopmost()).toBe(false);
    expect(closed).toEqual(['settings']);
  });
});