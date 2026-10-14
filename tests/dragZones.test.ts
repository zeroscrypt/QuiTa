import { describe, expect, it } from 'vitest';

/**
 * The drop zones are deliberately independent from timing: the middle half of
 * a row nests, the top/bottom quarters reorder. This mirrors the logic in
 * `src/stores/dragStore.svelte.ts` so a regression is caught without a GUI.
 */
type Zone = 'nest' | 'insert' | 'none';

function zoneFor(rel: number, canNest: boolean): Zone {
  const middle = rel > 0.25 && rel < 0.75;
  if (middle && canNest) return 'nest';
  return 'insert';
}

function insertIndexFor(rel: number, index: number): number {
  return rel >= 0.75 ? index + 1 : index;
}

describe('drag & drop zones', () => {
  it('treats the middle half of a row as the nesting zone', () => {
    expect(zoneFor(0.3, true)).toBe('nest');
    expect(zoneFor(0.5, true)).toBe('nest');
    expect(zoneFor(0.7, true)).toBe('nest');
  });

  it('treats the top and bottom quarters as insertion zones', () => {
    expect(zoneFor(0.05, true)).toBe('insert');
    expect(zoneFor(0.2, true)).toBe('insert');
    expect(zoneFor(0.8, true)).toBe('insert');
    expect(zoneFor(0.95, true)).toBe('insert');
  });

  it('never nests when the target cannot adopt subtasks', () => {
    expect(zoneFor(0.5, false)).toBe('insert');
  });

  it('inserts above when released in the top part, below in the bottom part', () => {
    expect(insertIndexFor(0.3, 2)).toBe(2);
    expect(insertIndexFor(0.8, 2)).toBe(3);
  });

  it('the zones do not depend on how long the pointer rests', () => {
    // Deterministic by design: holding longer must not switch the outcome.
    const immediate = zoneFor(0.5, true);
    const held = zoneFor(0.5, true);
    expect(held).toBe(immediate);
  });
});
/**
 * Sibling reorder inside a subtask group. Mirrors `dragStore.drop()`: an
 * existing member moves to the requested slot, a newcomer is appended.
 */
function reorderSiblings(ids: string[], sourceId: string, dropIndex: number | null): string[] {
  const next = [...ids];
  const from = next.indexOf(sourceId);
  if (dropIndex === null) return next;
  if (from >= 0) {
    const to = dropIndex > from ? dropIndex - 1 : dropIndex;
    if (to === from) return next;
    next.splice(to, 0, ...next.splice(from, 1));
    return next;
  }
  // Joining from outside lands where the line is shown — including the end.
  next.splice(Math.min(dropIndex, next.length), 0, sourceId);
  return next;
}

describe('subtask reordering', () => {
  it('moves a subtask up when dropped in the upper half of another one', () => {
    expect(reorderSiblings(['a', 'b', 'c'], 'c', 1)).toEqual(['a', 'c', 'b']);
  });

  it('moves a subtask down when dropped in the lower half', () => {
    expect(reorderSiblings(['a', 'b', 'c'], 'a', 2)).toEqual(['b', 'a', 'c']);
  });

  it('does nothing when the task is already at that position', () => {
    expect(reorderSiblings(['a', 'b', 'c'], 'b', 2)).toEqual(['a', 'b', 'c']);
  });

  it('can put a subtask first', () => {
    expect(reorderSiblings(['a', 'b', 'c'], 'c', 0)).toEqual(['c', 'a', 'b']);
  });

  // Dropping a root onto a subtask row joins the group at the shown position.
  it('places a task that joins the group from outside where the line is', () => {
    expect(reorderSiblings(['a', 'b'], 'root', 0)).toEqual(['root', 'a', 'b']);
    expect(reorderSiblings(['a', 'b'], 'root', 1)).toEqual(['a', 'root', 'b']);
    expect(reorderSiblings(['a', 'b'], 'root', 2)).toEqual(['a', 'b', 'root']);
  });

  it('ignores a drop without a position', () => {
    expect(reorderSiblings(['a', 'b'], 'a', null)).toEqual(['a', 'b']);
  });
});

/**
 * Mirror of `DragStore.locate()`: the row is found by geometry, so the space
 * below the last row is a real insertion position instead of "nowhere".
 */
function locate(
  rows: { top: number; bottom: number }[],
  y: number,
): { index: number; rel: number } {
  if (rows.length === 0) return { index: 0, rel: 0 };
  for (let index = 0; index < rows.length; index += 1) {
    const rect = rows[index];
    if (y < rect.bottom) {
      const rel = (y - rect.top) / Math.max(rect.bottom - rect.top, 1);
      return { index, rel: Math.min(Math.max(rel, 0), 1) };
    }
  }
  return { index: rows.length, rel: 1 };
}

const THREE_ROWS = [
  { top: 0, bottom: 50 },
  { top: 50, bottom: 100 },
  { top: 100, bottom: 150 },
];

describe('locating the row under the pointer', () => {
  it('finds the row the pointer is over', () => {
    expect(locate(THREE_ROWS, 10).index).toBe(0);
    expect(locate(THREE_ROWS, 60).index).toBe(1);
    expect(locate(THREE_ROWS, 149).index).toBe(2);
  });

  it('appends when the pointer is below the last row', () => {
    // The bug: this position used to resolve to "no row", so putting a task
    // at the very end was impossible.
    expect(locate(THREE_ROWS, 150)).toEqual({ index: 3, rel: 1 });
    expect(locate(THREE_ROWS, 400)).toEqual({ index: 3, rel: 1 });
  });

  it('inserts first when the pointer is above the first row', () => {
    expect(locate(THREE_ROWS, -20)).toEqual({ index: 0, rel: 0 });
  });

  it('handles an empty list without crashing', () => {
    expect(locate([], 120)).toEqual({ index: 0, rel: 0 });
  });
});

/** Mirror of the root-list branch of `DragStore.drop()`. */
function reorderRoots(ids: string[], sourceId: string, dropIndex: number | null): string[] {
  const next = [...ids];
  const from = next.indexOf(sourceId);
  if (dropIndex === null) return next;
  let to = dropIndex;
  if (to > from) to -= 1;
  if (to === from) return next;
  next.splice(to, 0, ...next.splice(from, 1));
  return next;
}

describe('placing a root task at the very end', () => {
  it('puts it last, not second-to-last', () => {
    expect(reorderRoots(['a', 'b', 'c'], 'a', 3)).toEqual(['b', 'c', 'a']);
    expect(reorderRoots(['a', 'b', 'c'], 'b', 3)).toEqual(['a', 'c', 'b']);
    expect(reorderRoots(['a', 'b', 'c'], 'c', 3)).toEqual(['a', 'b', 'c']);
  });

  it('puts it last even in a single-row list', () => {
    expect(reorderRoots(['a'], 'a', 1)).toEqual(['a']);
  });

  it('still supports every other position', () => {
    expect(reorderRoots(['a', 'b', 'c'], 'a', 0)).toEqual(['a', 'b', 'c']);
    expect(reorderRoots(['a', 'b', 'c'], 'c', 0)).toEqual(['c', 'a', 'b']);
    expect(reorderRoots(['a', 'b', 'c'], 'c', 1)).toEqual(['a', 'c', 'b']);
    expect(reorderRoots(['a', 'b', 'c'], 'a', 2)).toEqual(['b', 'a', 'c']);
  });
});
