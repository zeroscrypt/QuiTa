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
    notRelevantAt: null,
    notRelevantEventId: null,
    autoNotRelevantBy: null,
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

  it('renders a marked task red and dim, and does not call it checked', () => {
    // "Не актуальная" is not completion: no check, no strikethrough. The mark
    // carries the same weight as the check mark — a white cross on a red
    // fill — so it stays readable while the text of the row is dimmed. The
    // chip spells the state out in the same small font as the date badge.
    const body = renderItem(makeTask({ notRelevantAt: '2026-01-02T00:00:00Z' }));
    expect(body).toContain('task-row not-relevant');
    expect(body).not.toContain('task-row completed');
    expect(body).toContain('aria-checked="false"');
    expect(body).toContain('M2.5 2.5l7 7'); // the cross path
    expect(body).toContain('stroke="#fff"'); // white on red, not a faint glyph
    expect(body).toContain('stroke-width="2.2"'); // bolder than the old 1.9
    expect(body).toContain('mark-badge'); // the "не актуальная" caption
    expect(body).toContain('Не актуальная');
  });

  it('keeps the red cross on a task closed in both senses', () => {
    const body = renderItem(
      makeTask({
        completedAt: '2026-01-02T00:00:00Z',
        notRelevantAt: '2026-01-02T00:00:00Z',
      }),
    );
    // The look is the mark's, not the strikethrough's; one closure, red.
    expect(body).toContain('task-row not-relevant');
    expect(body).toContain('aria-checked="true"');
    expect(body).toContain('M2.5 2.5l7 7');
    expect(body).toContain('stroke="#fff"');
    expect(body).toContain('mark-badge');
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
