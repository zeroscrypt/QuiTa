import { describe, expect, it } from 'vitest';
import { buildDeletedHierarchy, retentionVerdict } from '../src/lib/trash';
import type { Task } from '../src/lib/types';

function task(id: string, parentId: string | null, sortOrder: number): Task {
  return {
    id,
    parentId,
    tabId: 'tab-1',
    title: `Задача ${id}`,
    createdAt: '2026-10-14T00:00:00Z',
    updatedAt: '2026-10-14T00:00:00Z',
    completedAt: null,
    deletedAt: '2026-10-14T00:00:00Z',
    dueDate: null,
    comment: null,
    sortOrder,
    completionEventId: null,
    autoCompletedBy: null,
  };
}

describe('buildDeletedHierarchy', () => {
  it('places every subtask right below its deleted parent', () => {
    const parent = task('a', null, -2);
    const first = task('a1', 'a', 0);
    const second = task('a2', 'a', 1);
    const other = task('b', null, -1);

    // Deliberately shuffled: the trash arrives ordered by deletion time.
    const nodes = buildDeletedHierarchy([first, other, second, parent]);

    // 'a' has the lower sort order, so it comes first despite arriving later.
    expect(nodes.map((n) => n.task.id)).toEqual(['a', 'a1', 'a2', 'b']);
    expect(nodes.map((n) => n.depth)).toEqual([0, 1, 1, 0]);
  });

  it('reports how many subtasks each parent carries', () => {
    const nodes = buildDeletedHierarchy([
      task('a', null, 0),
      task('a1', 'a', 0),
      task('a2', 'a', 1),
      task('b', null, 1),
    ]);

    expect(nodes[0]).toMatchObject({ depth: 0, childCount: 2, orphaned: false });
    expect(nodes[1]).toMatchObject({ depth: 1, childCount: 0, orphaned: false });
    expect(nodes[3]).toMatchObject({ depth: 0, childCount: 0, orphaned: false });
  });

  it('keeps an orphaned subtask visible as a top-level row', () => {
    // The parent is not in the trash — it was restored or purged on its own.
    const nodes = buildDeletedHierarchy([task('child', 'gone-parent', 0)]);

    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toMatchObject({ depth: 0, orphaned: true });
  });

  it('sorts siblings by their order instead of the arrival order', () => {
    const nodes = buildDeletedHierarchy([
      task('a', null, 0),
      task('a3', 'a', 2),
      task('a1', 'a', 0),
      task('a2', 'a', 1),
    ]);

    expect(nodes.map((n) => n.task.id)).toEqual(['a', 'a1', 'a2', 'a3']);
  });

  it('does not lose a task when the data is malformed', () => {
    // A subtask of a subtask cannot happen in QuiTa; it must still be listed.
    const nodes = buildDeletedHierarchy([
      task('root', null, 0),
      task('mid', 'root', 0),
      task('deep', 'mid', 0),
    ]);

    // 'mid' nests under 'root' as usual; 'deep', whose parent cannot host a
    // child, is shown at the top level marked as a former subtask — nothing
    // is lost and nothing is drawn as a level that does not exist.
    expect(nodes.map((n) => n.task.id)).toEqual(['root', 'mid', 'deep']);
    expect(nodes.map((n) => n.depth)).toEqual([0, 1, 0]);
    expect(nodes.map((n) => n.orphaned)).toEqual([false, false, true]);
  });

  it('handles an empty trash', () => {
    expect(buildDeletedHierarchy([])).toEqual([]);
  });
});


  describe('retentionVerdict', () => {
    const now = Date.parse('2026-10-17T12:00:00Z');
    const daysAgo = (n: number) => new Date(now - n * 86_400_000).toISOString();

    it('never removes anything while the setting is empty', () => {
      // The default is "no retention chosen", and a deletion the user never
      // agreed to must not follow from a field they never filled in.
      expect(retentionVerdict(daysAgo(400), null, now)).toEqual({ kind: 'never' });
      expect(retentionVerdict(null, null, now)).toEqual({ kind: 'never' });
    });

    it('counts the days down from the moment the task was thrown away', () => {
      expect(retentionVerdict(daysAgo(10), 30, now)).toEqual({
        kind: 'inDays',
        days: 20,
      });
      expect(retentionVerdict(daysAgo(0), 30, now)).toEqual({
        kind: 'inDays',
        days: 30,
      });
    });

    it('treats an overdue row as leaving on the next launch', () => {
      expect(retentionVerdict(daysAgo(30), 30, now)).toEqual({ kind: 'nextRun' });
      expect(retentionVerdict(daysAgo(31), 30, now)).toEqual({ kind: 'nextRun' });
    });

    it('does not invent a deadline from a missing or broken timestamp', () => {
      // A row with nothing to count from cannot promise time, but the rule is
      // on — so it leaves on the next launch instead of claiming to be safe.
      expect(retentionVerdict(null, 30, now)).toEqual({ kind: 'nextRun' });
      expect(retentionVerdict('не дата', 30, now)).toEqual({ kind: 'nextRun' });
    });
  });
