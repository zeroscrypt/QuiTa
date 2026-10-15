import { untrack } from 'svelte';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { SearchResult, Task, TaskId, TaskNode } from '$lib/types';
import * as taskService from '$services/taskService';
import { settingsStore } from './settingsStore.svelte';
import { tabStore } from './tabStore.svelte';
import { uiStore } from './uiStore.svelte';

class TaskStore {
  nodes = $state<TaskNode[]>([]);
  loading = $state(true);
  saving = $state(false);
  error = $state<string | null>(null);

  searchQuery = $state('');
  searchResult = $state<SearchResult | null>(null);
  searching = $state(false);

  expanded = $state<ReadonlySet<string>>(new Set<string>());
  /** Explicitly closed tasks, used while "always expand" is on. */
  collapsed = $state<ReadonlySet<string>>(new Set<string>());
  deleted = $state<Task[]>([]);

  /**
   * Tab badges count every task of a tab, including rows the current filter
   * hides, so they cannot be derived from `nodes`. Any operation that changes
   * how many tasks a tab holds — or how many are done — must call this.
   */
  private touchTabCounts(): void {
    void tabStore.refreshStats();
  }

  private errorTimer: ReturnType<typeof setTimeout> | null = null;
  private searchRequestId = 0;
  private unlistenShortcut: UnlistenFn | null = null;

  async init(): Promise<void> {
    // Apply the retention policy before the first render so the list is correct.
    if (settingsStore.autoDeleteDays !== null) {
      try {
        await taskService.purgeCompletedTasks();
      } catch (e) {
        this.showError(e);
      }
    }
    await this.load();
    // The Rust layer emits this when the global shortcut fires; the UI then
    // focuses the quick-add input.
    this.unlistenShortcut?.();
    this.unlistenShortcut = await listen('quita:global-shortcut', () => {
      window.dispatchEvent(new CustomEvent('quita:focus-add'));
    });
  }

  async load(): Promise<void> {
    // Read through `untrack`: callers are `$effect`s that declare their own
    // dependencies, and an implicit read here would make every preference
    // change silently reload the list.
    const includeCompleted = untrack(() => settingsStore.showCompleted);
    const tabId = untrack(() => tabStore.activeTabId);
    this.loading = true;
    try {
      this.nodes = await taskService.listTasks(includeCompleted, tabId);
      // Tab badges count every task of the tab, including the ones this
      // filter hides, so they are fetched separately.
      this.touchTabCounts();
      // Drop expansion state for tasks that no longer exist, so the set does
      // not grow for the whole life of the session.
      if (this.expanded.size > 0) {
        const visible = new Set<string>();
        for (const node of this.nodes) {
          visible.add(node.task.id);
          for (const child of node.children) visible.add(child.id);
        }
        const pruned = new Set([...this.expanded].filter((id) => visible.has(id)));
        const prunedCollapsed = new Set([...this.collapsed].filter((id) => visible.has(id)));
        if (pruned.size !== this.expanded.size) this.expanded = pruned;
        if (prunedCollapsed.size !== this.collapsed.size) this.collapsed = prunedCollapsed;
      }
      this.error = null;
    } catch (e) {
      this.showError(e);
    } finally {
      this.loading = false;
    }
  }

  async reload(): Promise<void> {
    await this.load();
  }

  async addRoot(title: string): Promise<boolean> {
    try {
      const task = await taskService.createTask(
        title,
        null,
        untrack(() => tabStore.activeTabId),
      );
      // The database decides the position; the optimistic insert mirrors the
      // "newest first" preference so the row does not jump after the reload.
      const newestFirst = untrack(() => settingsStore.newestFirst);
      this.nodes = (newestFirst
        ? [{ task, children: [] }, ...this.nodes]
        : [...this.nodes, { task, children: [] }]
      ).sort((a, b) => a.task.sortOrder - b.task.sortOrder);
      this.touchTabCounts();
      return true;
    } catch (e) {
      this.showError(e);
      return false;
    }
  }

  async addSubtask(parentId: TaskId, title: string): Promise<boolean> {
    try {
      const task = await taskService.createTask(title, parentId, null);
      // New subtasks go to the bottom of the list, matching the order the
      // database assigns (after the existing ones) and the drag & drop order.
      this.nodes = this.nodes.map((node) =>
        node.task.id === parentId
          ? { task: node.task, children: [...node.children, task] }
          : node,
      );
      this.touchTabCounts();
      return true;
    } catch (e) {
      this.showError(e);
      return false;
    }
  }

  async rename(id: TaskId, title: string): Promise<void> {
    try {
      const updated = await taskService.renameTask(id, title);
      this.applyUpdated([updated]);
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  /**
   * Applies the "newest first" switch to tasks that already exist: the stored
   * order is reversed in the database, so the list visibly turns over and the
   * result survives a restart.
   */
  async reverseRootOrder(): Promise<void> {
    try {
      await taskService.reverseRootOrder();
      await this.load();
    } catch (e) {
      this.showError(e);
    }
  }

  /**
   * Nests a task that carries subtasks: with only two levels available the
   * children cannot follow it, so they become siblings under the new parent.
   * Returns the number of tasks that were flattened, or 0 when the task had no
   * subtasks (then a plain reparent is enough).
   */
  async nestWithChildren(id: TaskId, parentId: TaskId): Promise<number> {
    const node = this.nodes.find((n) => n.task.id === id);
    const childCount = node?.children.length ?? 0;
    try {
      if (childCount === 0) {
        await taskService.reparentTask(id, parentId, false);
        await this.load();
        return 0;
      }
      const receipt = await taskService.nestAndFlatten(id, parentId, true);
      await this.load();
      this.showFlattenedParents(parentId, id);
      uiStore.announceFlatten(receipt, parentId);
      return receipt.flattened.length;
    } catch (e) {
      this.showError(e);
      await this.load();
      return 0;
    }
  }

  /** Opens the chain the task was moved into, so the result is visible. */
  private showFlattenedParents(parentId: TaskId, movedId: TaskId): void {
    const expanded = new Set(this.expanded);
    const collapsed = new Set(this.collapsed);
    expanded.add(parentId);
    expanded.add(movedId);
    collapsed.delete(parentId);
    collapsed.delete(movedId);
    this.expanded = expanded;
    this.collapsed = collapsed;
  }

  async toggle(id: TaskId): Promise<void> {
    try {
      // A toggle may complete/reopen a parent and its children at once, and may
      // bring tasks back into the visible list. Updating the returned rows in
      // place keeps the DOM (and the scroll position) stable instead of a full
      // reload that would jump back to the top of the list.
      const outcome = await taskService.toggleTaskCompletion(id, settingsStore.cascadeComplete);

      const known = new Set<string>();
      for (const node of this.nodes) {
        known.add(node.task.id);
        for (const child of node.children) known.add(child.id);
      }
      // A reopened parent may not be in the list yet — fetch its children.
      const missingRoots = outcome.updated.filter(
        (t) => t.parentId === null && !known.has(t.id),
      );

      this.applyUpdated(outcome.updated);

      if (missingRoots.length > 0) {
        for (const task of missingRoots) {
          if (task.deletedAt === null && (settingsStore.showCompleted || task.completedAt === null)) {
            let children: Task[] = [];
            try {
              children = await taskService.listSubtasks(task.id);
            } catch {
              children = [];
            }
            this.nodes = [...this.nodes, { task, children }].sort(
              (a, b) => a.task.sortOrder - b.task.sortOrder,
            );
          }
        }
      }
      // Completing or reopening changes the done/total badge of the tab.
      this.touchTabCounts();
    } catch (e) {
      // This is the action taken on every single click, and it was the one
      // left without a handler: a rejected call became an unhandled promise,
      // and the checkbox sprang back with nothing on screen to explain it.
      this.showError(e);
      await this.load();
    }
  }

  async setDueDate(id: TaskId, dueDate: string | null): Promise<void> {
    try {
      const updated = await taskService.setTaskDueDate(id, dueDate);
      this.applyUpdated([updated]);
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  /** Saves or clears a task comment. */
  async setComment(id: TaskId, comment: string | null): Promise<void> {
    try {
      const updated = await taskService.setTaskComment(id, comment);
      this.applyUpdated([updated]);
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  async remove(id: TaskId): Promise<void> {
    try {
      const deletedTasks = await taskService.softDeleteTask(id);
      const ids = new Set(deletedTasks.map((t) => t.id));
      // The backend cascades, so the answer contains the task *and* its
      // subtasks. Deleting a subtask must remove it from its parent's child
      // list, not just drop a root node.
      this.nodes = this.nodes
        .map((node) => ({
          task: node.task,
          children: node.children.filter((child) => !ids.has(child.id)),
        }))
        .filter((node) => !ids.has(node.task.id));
      const expanded = new Set(this.expanded);
      const collapsed = new Set(this.collapsed);
      for (const removed of ids) {
        expanded.delete(removed);
        collapsed.delete(removed);
      }
      this.expanded = expanded;
      this.collapsed = collapsed;
      this.touchTabCounts();
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  async restore(id: TaskId): Promise<void> {
    try {
      // The backend returns everything that came back: the task itself plus
      // the subtasks that were removed with it, or the task itself promoted to
      // a root when its parent is still in the trash.
      const restored = await taskService.restoreTask(id);
      const task = restored[0];
      if (!task) {
        await this.load();
        return;
      }
      const children = restored.slice(1);
      const node: TaskNode = {
        task,
        children: children.filter((child) => child.parentId === task.id),
      };
      this.nodes = [...this.nodes, node].sort(
        (a, b) => a.task.sortOrder - b.task.sortOrder,
      );
      // A subtask promoted to a root arrives with its subtasks attached.
      this.touchTabCounts();
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  async move(id: TaskId, direction: 'up' | 'down'): Promise<void> {
    try {
      await taskService.moveTask(id, direction);
    } catch (e) {
      this.showError(e);
    }
    await this.load();
  }

  /**
   * Makes a task a subtask of `parentId`, or a root task when `null`.
   * `placeAtTop` is false for drag & drop, so the task joins the end of the
   * subtask list instead of jumping to the top.
   */
  async reparent(id: TaskId, parentId: TaskId | null, placeAtTop = true): Promise<void> {
    try {
      await taskService.reparentTask(id, parentId, placeAtTop);
      await this.load();
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  /** Moves a task (and its subtasks) to another tab. */
  async moveToTab(id: TaskId, tabId: TaskId): Promise<void> {
    try {
      await taskService.moveTaskToTab(id, tabId);
      this.touchTabCounts();
      await this.load();
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  /** Persists a drag & drop order for the visible roots. */
  async reorder(orderedIds: TaskId[]): Promise<void> {
    try {
      await taskService.reorderTasks(orderedIds);
      const position = new Map(orderedIds.map((id, index) => [id, index * 10]));
      this.nodes = [...this.nodes].sort(
        (a, b) =>
          (position.get(a.task.id) ?? a.task.sortOrder) -
          (position.get(b.task.id) ?? b.task.sortOrder),
      );
    } catch (e) {
      this.showError(e);
      await this.load();
    }
  }

  async search(query: string): Promise<void> {
    this.searchQuery = query;
    const trimmed = query.trim();
    if (trimmed.length === 0) {
      this.searchResult = null;
      this.searching = false;
      return;
    }
    const tabId = untrack(() => tabStore.activeTabId);
    // Guard against out-of-order responses while typing fast.
    const requestId = ++this.searchRequestId;
    this.searching = true;
    try {
      const result = await taskService.searchTasks(trimmed, tabId);
      if (requestId !== this.searchRequestId) return;
      this.searchResult = result;
    } catch (e) {
      if (requestId !== this.searchRequestId) return;
      this.showError(e);
      this.searchResult = null;
    } finally {
      if (requestId === this.searchRequestId) this.searching = false;
    }
  }

  clearSearch(): void {
    this.searchQuery = '';
    this.searchResult = null;
    this.searching = false;
    this.searchRequestId++;
  }

  /**
   * Re-runs the active query against the current tab. Does nothing when the
   * search is empty — the list is reloaded separately, and doing it here too
   * would issue two `list_tasks` calls per tab switch.
   */
  async refreshSearch(): Promise<void> {
    const query = untrack(() => this.searchQuery);
    if (query.trim().length > 0) await this.search(query);
  }

  /**
 * Expansion state depends on the "always expand subtasks" preference:
 *  - preference on  → everything is open unless the id is in `collapsed`
 *  - preference off → only ids in `expanded` are open
 * Keeping two sets means a user who turns the preference off still finds the
 * tasks they had opened, and vice versa.
 */
toggleExpanded(id: TaskId): void {
    const global = settingsStore.expandSubtasks;
    const source = global ? this.collapsed : this.expanded;
    const next = new Set(source);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    if (global) this.collapsed = next;
    else this.expanded = next;
  }

  isExpanded(id: TaskId): boolean {
    return settingsStore.expandSubtasks ? !this.collapsed.has(id) : this.expanded.has(id);
  }

  /** Opens every task that has subtasks. */
  expandAll(): void {
    const ids = this.nodes.filter((node) => node.children.length > 0).map((n) => n.task.id);
    this.expanded = new Set(ids);
    this.collapsed = new Set();
  }

  /** Closes every task, overriding the "always expand" preference. */
  collapseAll(): void {
    this.collapsed = new Set(this.nodes.filter((n) => n.children.length > 0).map((n) => n.task.id));
    this.expanded = new Set();
  }

  async loadDeleted(): Promise<void> {
    try {
      this.deleted = await taskService.listDeletedTasks();
    } catch (e) {
      this.showError(e);
    }
  }

  private applyUpdated(updated: Task[]): void {
    const byId = new Map<string, Task>(updated.map((t) => [t.id, t]));
    const showCompleted = settingsStore.showCompleted;
    this.nodes = this.nodes
      .map((node) => {
        const newTask = byId.get(node.task.id);
        const children = node.children.map((c) => byId.get(c.id) ?? c);
        return { task: newTask ?? node.task, children };
      })
      .filter((node) => node.task.deletedAt === null)
      // Completed tasks disappear from the list unless the user opted in.
      .filter((node) => showCompleted || node.task.completedAt === null);
  }

  showError(e: unknown): void {
    const message = e instanceof Error ? e.message : String(e);
    this.error = message;
    if (this.errorTimer) clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => {
      this.error = null;
    }, 6000);
  }
}

export const taskStore = new TaskStore();
