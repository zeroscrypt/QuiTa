import type { Task } from './types';

/** One row of the recycle bin: the task plus where it sits in the hierarchy. */
export interface DeletedNode {
  task: Task;
  /** 0 — a root task, 1 — a subtask of the row right above it. */
  depth: 0 | 1;
  /** Number of subtasks listed right below this row. */
  childCount: number;
  /** The task used to live under a parent that is not in the trash. */
  orphaned: boolean;
}

/**
 * Builds the flat row list for the recycle bin: every parent is immediately
 * followed by its subtasks, so the hierarchy is visible instead of guessed.
 *
 * A subtask whose parent is missing from `tasks` (the parent was restored on
 * its own, or purged) stays a top-level row marked as a former subtask —
 * hiding it or nesting it wrongly would both be a lie.
 *
 * Only one level exists in QuiTa, so `depth` is never above 1; a malformed
 * chain (a subtask of a subtask) degrades to a top-level row rather than
 * disappearing.
 */
export function buildDeletedHierarchy(tasks: Task[]): DeletedNode[] {
  const present = new Set(tasks.map((task) => task.id));
  const bySort = (a: Task, b: Task) => a.sortOrder - b.sortOrder;

  // Only a root task may act as a container. Nesting a subtask under a subtask
  // would make the grandchild unreachable in the rendering, so such a row is
  // shown at the top level instead — the list must never lose a task.
  const containerIds = new Set(
    tasks.filter((task) => task.parentId === null && present.has(task.id)).map((t) => t.id),
  );

  const children = new Map<string, Task[]>();
  const roots: Task[] = [];
  for (const task of tasks) {
    if (task.parentId !== null && containerIds.has(task.parentId)) {
      const list = children.get(task.parentId) ?? [];
      list.push(task);
      children.set(task.parentId, list);
    } else {
      roots.push(task);
    }
  }

  const nodes: DeletedNode[] = [];
  for (const root of roots.sort(bySort)) {
    const kids = (children.get(root.id) ?? []).sort(bySort);
    nodes.push({
      task: root,
      depth: 0,
      childCount: kids.length,
      orphaned: root.parentId !== null,
    });
    for (const kid of kids) {
      nodes.push({ task: kid, depth: 1, childCount: 0, orphaned: false });
    }
  }
  return nodes;
}