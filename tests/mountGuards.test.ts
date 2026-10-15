import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

/**
 * Dialogs are mounted behind a condition in App.svelte while the state they
 * render lives in the store. Nothing connects the two at compile time, and a
 * store flag that no condition mentions renders nothing at all — the tab
 * deletion confirmation was exactly that, and the menu item did nothing.
 */

const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');

const app = read('../src/App.svelte');
const uiStore = read('../src/stores/uiStore.svelte.ts');

/** Every confirmation flag the store exposes. */
function confirmationStates(): string[] {
  const names = new Set<string>();
  const pattern = /^\s{2}(confirm[A-Z]\w*)\s*=\s*\$state/gm;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(uiStore)) !== null) names.add(match[1]);
  return [...names];
}

describe('dialog mount points', () => {
  const states = confirmationStates();

  it('finds the confirmation states', () => {
    // If the pattern stops matching, the test below would pass vacuously.
    expect(states).toContain('confirmDelete');
  });

  it('mounts a dialog for every confirmation state', () => {
    for (const name of states) {
      expect(app, `App.svelte never mentions ${name}, so its dialog cannot open`).toContain(
        `uiStore.${name}`,
      );
    }
  });
});