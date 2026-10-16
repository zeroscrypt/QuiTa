import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { UiStore } from '../src/stores/uiStore.svelte';
import type { Tab, Task } from '../src/lib/types';

/**
 * One dialog asks two different questions — deleting a task and deleting a tab —
 * and they live in two state fields. Every way out of the dialog has to clear
 * both.
 *
 * The Cancel button cleared only the task field, so on the tab question it did
 * nothing at all: the dialog stayed on screen and the only way out was to agree
 * to delete the tab. Nothing failed — the button simply had no effect, which is
 * exactly the class of bug a type checker cannot see and a rendering test would
 * only catch if it went looking for that one button.
 *
 * The two half-cancels are gone rather than fixed, so the mistake cannot be made
 * again by picking the wrong one; the source check keeps a third one from
 * appearing.
 */

const DIALOG = 'src/components/ConfirmDialog.svelte';

const read = (path: string) => readFileSync(path, 'utf8');

/** Every source file under `src`, since the Cancel call could move anywhere. */
function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(full);
    return /\.(svelte|ts)$/.test(entry.name) ? [full] : [];
  });
}

const tab = { id: 'tab-1', name: 'Работа' } as unknown as Tab;
const task = { id: 'task-1', title: 'Задача' } as unknown as Task;

describe('closing the confirmation dialog', () => {
  it('dismisses the question about a tab', () => {
    const store = new UiStore();
    store.askDeleteTab(tab, 3);
    expect(store.confirmTabDelete).not.toBeNull();

    store.cancelConfirm();
    expect(store.confirmTabDelete).toBeNull();
  });

  it('dismisses the question about a task', () => {
    const store = new UiStore();
    store.askDelete(task, 2);
    expect(store.confirmDelete).not.toBeNull();

    store.cancelConfirm();
    expect(store.confirmDelete).toBeNull();
  });

  it('leaves no way to cancel only one half of the dialog', () => {
    const halves = sourceFiles('src').filter((file) =>
      /\.cancel(?:Delete|TabDelete)\(\)/.test(read(file)),
    );
    expect(halves).toEqual([]);
  });

  it('sends the Cancel button through the same method as the other exits', () => {
    const source = read(DIALOG);
    const button = source
      .split('\n')
      .find((line) => line.includes("t('delete.cancel')"));
    expect(button, 'the Cancel button must exist').toBeDefined();
    expect(button).toContain('uiStore.cancelConfirm()');
  });

  it('closes both questions on every exit from the dialog', () => {
    // Escape, the backdrop and Enter all end in the layer's close callback, so
    // counting the method instead of each call site covers them at once.
    const source = read(DIALOG);
    expect(source.match(/uiStore\.cancelConfirm\(\)/g)?.length).toBe(4);
  });
});
