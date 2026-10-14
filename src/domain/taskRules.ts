import type { Task, TaskNode } from '$lib/types';

/** Builds the flat list into root nodes with children, sorted by sortOrder. */
export function buildTaskTree(nodes: TaskNode[]): TaskNode[] {
  return [...nodes].sort((a, b) => a.task.sortOrder - b.task.sortOrder);
}

export interface SubtaskStats {
  total: number;
  done: number;
}

export function getSubtaskStats(children: Task[]): SubtaskStats {
  return {
    total: children.length,
    done: children.filter((c) => c.completedAt !== null).length,
  };
}

/**
 * Progress badge of a task with subtasks.
 *
 * The task itself counts as one: a task with three subtasks is four tasks, so
 * the badge reads "3/4", never "2/3".
 */
export function formatProgress(stats: SubtaskStats, taskDone = false): string | null {
  if (stats.total === 0) return null;
  return `${stats.done + (taskDone ? 1 : 0)}/${stats.total + 1}`;
}

/**
 * A completed task that still has open subtasks: "partially done".
 *
 * This is a normal state (the parent is finished, the remaining work lives
 * below it), so it gets its own look instead of looking plain completed.
 */
export function isPartiallyDone(task: Task, children: Task[]): boolean {
  if (task.completedAt === null) return false;
  return children.some((child) => child.completedAt === null);
}

/** A task is overdue when its calendar date is before today (local time). */
export function isOverdue(task: Task, today: string = todayLocal()): boolean {
  if (!task.dueDate) return false;
  return task.dueDate < today;
}

function todayLocal(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/**
 * Only root tasks can receive subtasks (matches the Rust rules).
 *
 * A *completed* root may still get them: the row then reads as partially
 * done, which is how "ВМ поднята, осталось настроить фаервол" is expressed.
 */
export function canHaveSubtasks(task: Task): boolean {
  return task.parentId === null;
}

export function isCompleted(task: Task): boolean {
  return task.completedAt !== null;
}

/** Truncates titles for compact menus/tooltips. */
export function truncate(title: string, maxLength: number): string {
  return title.length > maxLength ? `${title.slice(0, maxLength - 1)}…` : title;
}
