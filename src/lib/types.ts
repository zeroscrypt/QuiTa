/** Mirrors `src-tauri/src/domain/task.rs`. Timestamps are RFC 3339 UTC. */

export type TaskId = string;

export interface Task {
  id: TaskId;
  parentId: TaskId | null;
  /** Tab this task belongs to. */
  tabId: TaskId | null;
  title: string;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  deletedAt: string | null;
  /** Calendar date YYYY-MM-DD, never a UTC timestamp. */
  dueDate: string | null;
  /** Free-form note. `null` when the task has no comment. */
  comment: string | null;
  sortOrder: number;
  /** Identifies the parent-completion event that completed this task's children. */
  completionEventId: string | null;
  /** For subtasks: completion event of the parent that auto-completed it. */
  autoCompletedBy: string | null;
  /** The task is kept on purpose but will not be done: "не актуальная". */
  notRelevantAt: string | null;
  /** Identifies the parent mark event that marked this task's children. */
  notRelevantEventId: string | null;
  /** For subtasks: mark event of the parent that auto-marked it. */
  autoNotRelevantBy: string | null;
}

/** A tab groups tasks by topic or project. */
export interface Tab {
  id: TaskId;
  name: string;
  /** Hex colour assigned automatically on creation. */
  color: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  /**
   * Set while the tab sits in the trash. The row survives so its tasks keep a
   * valid reference and can be brought back into the same tab.
   */
  deletedAt: string | null;
}

/**
 * What deleting a tab did, and what the undo needs. Nothing else can work this
 * out afterwards: once the tasks are in the trash they look like any others.
 */
export interface TabDeletion {
  tab: Tab;
  taskIds: TaskId[];
}

/**
 * What a delete did: the tasks it moved to the trash, and what fell out of the
 * trash because it was full.
 *
 * Both halves are reported because both are the user's business — the first is
 * visible in the trash, the second is gone for good.
 */
export interface DeleteOutcome {
  deleted: Task[];
  evicted: Task[];
}

export interface TaskNode {
  task: Task;
  children: Task[];
}

export interface MatchedSubtask {
  task: Task;
  parent: Task;
}

export interface SearchResult {
  roots: Task[];
  matchedSubtasks: MatchedSubtask[];
}

export interface AppSettings {
  /** "system" | "light" | "dark" */
  theme: "system" | "light" | "dark";
  showCompleted: boolean;
  /** Completing a parent also completes its open subtasks (default: true). */
  cascadeComplete: boolean;
  /** Completed tasks are soft-deleted after this many days (null = never). */
  autoDeleteDays: number | null;
  /**
   * How many soft-deleted tasks the trash keeps. Zero turns the rule off and
   * lets the trash grow without a bound.
   */
  trashLimit: number;
  /**
   * Days a tree may sit in the trash before it is removed for good (null =
   * never). Counted from the last moment the tree entered the trash.
   */
  trashRetentionDays: number | null;
  /** Interface language. */
  language: "ru" | "en";
  /** Subtasks are shown without being expanded by hand (default: true). */
  expandSubtasks: boolean;
  /** Show a done/total counter on every tab (default: true). */
  showTabCounts: boolean;
  /** New root tasks go to the top of the tab; false puts them at the bottom. */
  newestFirst: boolean;
}

export interface ToggleOutcome {
  updated: Task[];
}

/** Error payload surfaced by Tauri commands (see src-tauri/src/error.rs). */
export class TaskServiceError extends Error {
  constructor(
    message: string,
    public readonly cause?: unknown,
  ) {
    super(message);
    this.name = "TaskServiceError";
  }
}

/** A child's position before a flattening move, used to undo it. */
export interface ChildPlacement {
  id: TaskId;
  sortOrder: number;
}

/** Everything needed to undo a flatten-move and to describe it to the user. */
export interface ReparentReceipt {
  taskId: TaskId;
  previousParentId: TaskId | null;
  previousTabId: TaskId | null;
  previousSortOrder: number;
  flattened: ChildPlacement[];
}
