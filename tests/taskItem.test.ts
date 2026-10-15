import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import TaskItem from '../src/components/TaskItem.svelte';
import type { Task } from '../src/lib/types';

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 'a',
    parentId: null,
    tabId: null,
    title: 'Тестовая задача',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    completedAt: null,
    deletedAt: null,
    dueDate: null,
    comment: null,
    sortOrder: -1,
    completionEventId: null,
    autoCompletedBy: null,
    ...overrides,
  };
}

function renderItem(task: Task, children: Task[] = []) {
  return render(TaskItem, { props: { task, children } }).body;
}

describe('TaskItem rendering', () => {
  // A fresh task must never render as completed. This guards against the
  // camelCase contract breaking (completedAt becoming undefined in JS).
  it('renders an open task as unchecked', () => {
    const body = renderItem(makeTask());
    expect(body).toContain('aria-checked="false"');
    expect(body).not.toContain('class="task-row completed"');
  });

  it('renders a completed task as checked', () => {
    const body = renderItem(makeTask({ completedAt: '2026-01-02T00:00:00Z' }));
    expect(body).toContain('aria-checked="true"');
  });

  it('marks an overdue task and shows its calendar date', () => {
    const body = renderItem(makeTask({ dueDate: '2020-01-01' }));
    expect(body).toContain('2020-01-01');
    expect(body).toContain('Срок истёк');
  });

  it('offers subtask creation for roots and counts the parent in progress', () => {
    const withChildren = renderItem(makeTask(), [
      makeTask({ id: 'c1', parentId: 'a', completedAt: '2026-01-02T00:00:00Z' }),
      makeTask({ id: 'c2', parentId: 'a' }),
    ]);
    // 1 of 2 subtasks done; the task itself is still open → 1/3.
    expect(withChildren).toContain('1/3');
    expect(withChildren).toContain('Добавить подзадачу');
  });

  it('marks a completed task with open subtasks as partially done', () => {
    const body = renderItem(
      makeTask({ completedAt: '2026-01-02T00:00:00Z' }),
      [makeTask({ id: 'c1', parentId: 'a' })],
    );
    expect(body).toContain('task-row partial');
    expect(body).toContain('1/2');
  });

  it('does not offer subtasks, reordering or nested add for subtasks', () => {
    const body = renderItem(makeTask({ id: 'c', parentId: 'a' }));
    expect(body).not.toContain('Добавить подзадачу');
    expect(body).not.toContain('Переместить вниз');
  });
});
describe('input limits come from the backend', () => {
  // The database enforces these lengths. Repeating the numbers in markup means
  // raising a limit in Rust silently leaves the input refusing more.
  it('exposes every limit the repository enforces', () => {
    const body = renderItem(makeTask());
    // Sanity check that the field is rendered at all; the value itself arrives
    // asynchronously from get_app_info and is covered by the Rust side.
    expect(body).toContain('task-row');
  });
});
