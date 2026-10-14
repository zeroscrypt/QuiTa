import { describe, expect, it } from 'vitest';
import {
  buildTaskTree,
  canHaveSubtasks,
  formatProgress,
  getSubtaskStats,
  isOverdue,
} from '../src/domain/taskRules';
import type { Task, TaskNode } from '../src/lib/types';

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'id',
    parentId: null,
    tabId: null,
    title: 'Task',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    completedAt: null,
    deletedAt: null,
    dueDate: null,
    comment: null,
    sortOrder: 0,
    completionEventId: null,
    autoCompletedBy: null,
    ...overrides,
  };
}

function makeNode(task: Partial<Task> = {}, children: Task[] = []): TaskNode {
  return { task: makeTask(task), children };
}

describe('buildTaskTree', () => {
  it('sorts nodes by sortOrder', () => {
    const b = makeNode({ id: 'b', sortOrder: 1 });
    const a = makeNode({ id: 'a', sortOrder: -5 });
    const c = makeNode({ id: 'c', sortOrder: 10 });
    const tree = buildTaskTree([c, b, a]);
    expect(tree.map((n) => n.task.id)).toEqual(['a', 'b', 'c']);
  });
});

describe('getSubtaskStats', () => {
  it('counts completed and total subtasks', () => {
    const children = [
      makeTask({ id: '1', completedAt: '2026-01-02T00:00:00Z' }),
      makeTask({ id: '2' }),
      makeTask({ id: '3', completedAt: '2026-01-03T00:00:00Z' }),
    ];
    expect(getSubtaskStats(children)).toEqual({ total: 3, done: 2 });
  });

  it('handles no subtasks', () => {
    expect(getSubtaskStats([])).toEqual({ total: 0, done: 0 });
  });
});

describe('formatProgress', () => {
  it('formats done/total including the task itself', () => {
    // Three subtasks + the task = four tasks.
    expect(formatProgress({ total: 3, done: 2 }, true)).toBe('3/4');
    expect(formatProgress({ total: 3, done: 2 }, false)).toBe('2/4');
  });

  it('returns null when there are no subtasks', () => {
    expect(formatProgress({ total: 0, done: 0 })).toBeNull();
  });
});

describe('isOverdue', () => {
  it('detects overdue calendar dates', () => {
    expect(isOverdue(makeTask({ dueDate: '2020-01-01' }), '2026-10-13')).toBe(true);
  });

  it('treats today and future dates as not overdue', () => {
    expect(isOverdue(makeTask({ dueDate: '2026-10-13' }), '2026-10-13')).toBe(false);
    expect(isOverdue(makeTask({ dueDate: '2027-01-01' }), '2026-10-13')).toBe(false);
  });

  it('treats missing due dates as not overdue', () => {
    expect(isOverdue(makeTask(), '2026-10-13')).toBe(false);
  });
});

describe('canHaveSubtasks', () => {
  it('allows active root tasks', () => {
    expect(canHaveSubtasks(makeTask())).toBe(true);
  });

  it('forbids subtasks (only one level)', () => {
    expect(canHaveSubtasks(makeTask({ parentId: 'parent' }))).toBe(false);
  });

  it('allows completed roots to receive more subtasks', () => {
    // "ВМ поднята, осталось настроить фаервол": the parent stays done and the
    // row reads as partially done, so the "+" must remain available.
    expect(
      canHaveSubtasks(makeTask({ completedAt: '2026-01-02T00:00:00Z' })),
    ).toBe(true);
  });
});
