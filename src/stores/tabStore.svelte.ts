import type { Tab, TabDeletion, Task, TaskId } from "$lib/types";
import * as tabService from "$services/tabService";

class TabStore {
  tabs = $state<Tab[]>([]);
  activeTabId = $state<TaskId | null>(null);
  /** Shown as a toast; the tab strip has no other place to put a message. */
  error = $state<string | null>(null);

  /** Tab being renamed (double click) or created. */
  editingTabId = $state<TaskId | "new" | null>(null);
  /** done/total per tab, refreshed after every list load. */
  stats = $state<Record<TaskId, { done: number; total: number }>>({});

  async init(): Promise<void> {
    try {
      this.tabs = await tabService.listTabs();
    } catch {
      this.tabs = [];
    }
    if (this.tabs.length > 0) {
      const stored = localStorage.getItem("quita.activeTab");
      this.activeTabId = this.tabs.some((t) => t.id === stored)
        ? stored
        : this.tabs[0].id;
    }
  }

  /**
   * Re-reads the tab list after an import that may have brought new tabs,
   * keeping the current selection when that tab still exists.
   */
  async reload(): Promise<void> {
    const previous = this.activeTabId;
    try {
      this.tabs = await tabService.listTabs();
    } catch {
      return;
    }
    this.activeTabId = this.tabs.some((t) => t.id === previous)
      ? previous
      : null;
    if (this.activeTabId === null && this.tabs.length > 0)
      this.activeTabId = this.tabs[0].id;
  }

  /** Counter for a tab, or null when it has not been loaded yet. */
  statFor(tabId: TaskId): { done: number; total: number } | null {
    return this.stats[tabId] ?? null;
  }

  private statsRequest = 0;

  /**
   * Pulls fresh counters. Failures are silent because the badge is cosmetic,
   * and a guard drops out-of-order answers so a slow response cannot overwrite
   * a newer one after a quick burst of clicks.
   */
  async refreshStats(): Promise<void> {
    const request = ++this.statsRequest;
    try {
      const list = await tabService.tabStats();
      if (request !== this.statsRequest) return;
      const next: Record<TaskId, { done: number; total: number }> = {};
      for (const stat of list)
        next[stat.tabId] = { done: stat.done, total: stat.total };
      this.stats = next;
    } catch {
      // Keep the previous counters.
    }
  }

  get activeTab(): Tab | null {
    return this.tabs.find((t) => t.id === this.activeTabId) ?? null;
  }

  select(tabId: TaskId): void {
    if (this.tabs.some((t) => t.id === tabId)) {
      this.activeTabId = tabId;
      localStorage.setItem("quita.activeTab", tabId);
    }
  }

  async create(name?: string): Promise<Tab> {
    const tab = await tabService.createTab(name);
    this.tabs = [...this.tabs, tab];
    this.select(tab.id);
    return tab;
  }

  async rename(tabId: TaskId, name: string): Promise<void> {
    const trimmed = name.trim();
    if (!trimmed) return;
    const updated = await tabService.renameTab(tabId, trimmed);
    this.tabs = this.tabs.map((t) => (t.id === tabId ? updated : t));
  }

  /** Persists a new tab order (drag & drop). */
  async reorder(orderedIds: TaskId[]): Promise<void> {
    try {
      const tabs = await tabService.reorderTabs(orderedIds);
      this.tabs = tabs.length > 0 ? tabs : this.tabs;
    } catch {
      // Fall back to a local sort so the strip still reflects the drop.
      const position = new Map(orderedIds.map((id, index) => [id, index]));
      this.tabs = [...this.tabs].sort(
        (a, b) =>
          (position.get(a.id) ?? a.sortOrder) -
          (position.get(b.id) ?? b.sortOrder),
      );
    }
  }

  /**
   * Hides the tab and returns what the undo needs. Its tasks stay where they
   * are until the undo window closes — see `finalize`, which the notice calls
   * when the countdown reaches zero.
   *
   * The tasks are not handed to a neighbouring tab: that read as the tab's notes
   * quietly leaking into the tab on the left.
   */
  async remove(tabId: TaskId): Promise<TabDeletion | null> {
    if (this.tabs.length <= 1) return null; // the last tab is never removed
    const deletion = await tabService.deleteTab(tabId);
    this.tabs = this.tabs.filter((t) => t.id !== tabId);
    if (this.activeTabId === tabId) {
      this.select(this.tabs[0].id);
    }
    return deletion;
  }

  /**
   * The second half of the deletion: now the window has passed, the tab's tasks
   * go to the trash.
   *
   * Returns what the trash pushed out to stay within its bound, so the caller
   * can name it: a whole tab arriving at once is the fastest way past the
   * bound, and a task pushed out silently is a task gone for good.
   *
   * Safe to call twice — the second call finds no live tasks and returns
   * nothing — which matters because a timer that fires while the app is closing
   * can easily run twice.
   */
  async finalize(deletion: TabDeletion): Promise<Task[]> {
    const outcome = await tabService.finalizeTabDeletion(deletion.tab.id);
    return outcome.evicted;
  }

  /** Puts a hidden tab and its tasks back during the undo window. */
  async restore(deletion: TabDeletion): Promise<void> {
    await tabService.restoreDeletedTab(deletion.tab.id, deletion.taskIds);
    await this.reload();
    if (!this.tabs.some((tab) => tab.id === deletion.tab.id)) {
      this.tabs = [...this.tabs, deletion.tab];
    }
    this.select(deletion.tab.id);
  }
}

export const tabStore = new TabStore();
