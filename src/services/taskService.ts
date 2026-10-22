import { invoke } from '@tauri-apps/api/core';
import type {
  DeleteOutcome,
  ReparentReceipt,
  SearchResult,
  Task,
  TaskId,
  TaskNode,
  ToggleOutcome,
} from '$lib/types';

/** Typed wrappers around Tauri commands. All data access goes through here. */

export function createTask(
  title: string,
  parentId: TaskId | null = null,
  tabId: TaskId | null = null,
): Promise<Task> {
  return invoke<Task>('create_task', { title, parentId, tabId });
}

export function renameTask(id: TaskId, title: string): Promise<Task> {
  return invoke<Task>('rename_task', { id, title });
}

export function toggleTaskCompletion(
  id: TaskId,
  cascade: boolean,
): Promise<ToggleOutcome> {
  return invoke<ToggleOutcome>('toggle_task_completion', { id, cascadeComplete: cascade });
}

/**
 * Flips the "не актуальная" mark. The backend mirrors the completion cascade
 * on its own, so the UI only needs the id — no per-call settings.
 */
export function toggleTaskNotRelevance(id: TaskId): Promise<ToggleOutcome> {
  return invoke<ToggleOutcome>('toggle_task_not_relevance', { id });
}

export function setTaskDueDate(id: TaskId, dueDate: string | null): Promise<Task> {
  return invoke<Task>('set_task_due_date', { id, dueDate });
}

/** Sets or clears a task comment. Blank text is stored as `null`. */
export function setTaskComment(id: TaskId, comment: string | null): Promise<Task> {
  return invoke<Task>('set_task_comment', { id, comment });
}

export function softDeleteTask(id: TaskId): Promise<DeleteOutcome> {
  return invoke<DeleteOutcome>('soft_delete_task', { id });
}

export function restoreTask(id: TaskId): Promise<Task[]> {
  return invoke<Task[]>('restore_task', { id });
}

/** Permanently removes a soft-deleted task and its subtasks. */
export function purgeTask(id: TaskId): Promise<Task[]> {
  return invoke<Task[]>('purge_task', { id });
}

/** Permanently removes every soft-deleted task. */
export function purgeAllDeleted(): Promise<number> {
  return invoke<number>('purge_all_deleted');
}

/** Persists a new sibling order (drag & drop). */
export function reorderTasks(orderedIds: TaskId[]): Promise<void> {
  return invoke<void>('reorder_tasks', { orderedIds });
}

/** Drag & drop between levels; `null` parent promotes the task to root. */
export function reparentTask(
  id: TaskId,
  parentId: TaskId | null,
  placeAtTop = true,
): Promise<Task> {
  return invoke<Task>('reparent_task', { id, parentId, placeAtTop });
}

/** Drag & drop onto a tab. */
export function moveTaskToTab(id: TaskId, tabId: TaskId): Promise<Task> {
  return invoke<Task>('move_task_to_tab', { id, tabId });
}

export function moveTask(id: TaskId, direction: 'up' | 'down'): Promise<void> {
  return invoke<void>('move_task', { id, direction });
}

export function listTasks(
  includeCompleted: boolean,
  tabId: TaskId | null,
): Promise<TaskNode[]> {
  return invoke<TaskNode[]>('list_tasks', { includeCompleted, tabId });
}

/** A window of the trash plus how many tasks are in there altogether. */
export interface TrashPage {
  tasks: Task[];
  total: number;
}

export function listDeletedTasks(offset = 0): Promise<TrashPage> {
  return invoke<TrashPage>('list_deleted_tasks', { offset });
}

export function listSubtasks(parentId: TaskId): Promise<Task[]> {
  return invoke<Task[]>('list_subtasks', { parentId });
}

/**
 * Counts every active subtask, including the completed ones the list may be
 * hiding. The delete confirmation quotes this number.
 */
export function countSubtasks(parentId: TaskId): Promise<number> {
  return invoke<number>('count_subtasks', { parentId });
}

/** Turns the root task order over in every tab. */
export function reverseRootOrder(): Promise<void> {
  return invoke<void>('reverse_root_order');
}

/**
 * Nests a task that has subtasks: they become siblings under the new parent.
 * Only two levels exist, so the hierarchy cannot be kept.
 */
export function nestAndFlatten(
  taskId: TaskId,
  parentId: TaskId,
  placeAtBottom: boolean,
): Promise<ReparentReceipt> {
  return invoke<ReparentReceipt>('nest_and_flatten', { taskId, parentId, placeAtBottom });
}

/** Restores the structure a flatten-move produced. */
export function undoNest(receipt: ReparentReceipt): Promise<void> {
  return invoke<void>('undo_nest', { receipt });
}

/** Soft-deletes completed tasks older than the configured retention period. */
export function purgeCompletedTasks(): Promise<DeleteOutcome> {
  // Reports what the trash pushed out to stay within its bound: the timer
  // moves whole trees in, and a bound that only held for single deletions
  // would not be a bound.
  return invoke<DeleteOutcome>('purge_completed_tasks');
}

/** Removes trees that have sat in the trash longer than the configured period. */
export function purgeTrashByAge(): Promise<Task[]> {
  return invoke<Task[]>('purge_trash_by_age');
}

export function searchTasks(query: string, tabId: TaskId | null): Promise<SearchResult> {
  return invoke<SearchResult>('search_tasks', { query, tabId });
}
