import { invoke } from "@tauri-apps/api/core";
import type { DeleteOutcome, Tab, TabDeletion, TaskId } from "$lib/types";

export function listTabs(): Promise<Tab[]> {
  return invoke<Tab[]>("list_tabs");
}

export function createTab(name?: string): Promise<Tab> {
  return invoke<Tab>("create_tab", { name: name ?? null });
}

export function renameTab(id: TaskId, name: string): Promise<Tab> {
  return invoke<Tab>("rename_tab", { id, name });
}

export function reorderTabs(orderedIds: TaskId[]): Promise<Tab[]> {
  return invoke<Tab[]>("reorder_tabs", { orderedIds });
}

/** Done/total counters per tab. Subtasks count as tasks. */
export interface TabStat {
  tabId: TaskId;
  total: number;
  done: number;
}

export function tabStats(): Promise<TabStat[]> {
  return invoke<TabStat[]>("tab_stats");
}

/**
 * Hides a tab and remembers its tasks. The tasks stay in place until the undo
 * window closes — see `finalizeTabDeletion`, which is the second half.
 */
export function deleteTab(id: TaskId): Promise<TabDeletion> {
  return invoke<TabDeletion>("delete_tab", { id });
}

/** Sends a hidden tab's tasks to the trash; called when the window passes. */
export function finalizeTabDeletion(id: TaskId): Promise<DeleteOutcome> {
  // Reports what the trash pushed out: a whole tab arriving at once is the
  // fastest way past the bound, so the rule has to hold here too.
  return invoke<DeleteOutcome>("finalize_tab_deletion", { id });
}

/** Finishes a deletion whose window was cut short by the app closing. */
export function sweepPendingTabDeletions(): Promise<DeleteOutcome> {
  return invoke<DeleteOutcome>("sweep_pending_tab_deletions");
}

export function restoreDeletedTab(
  id: TaskId,
  taskIds: TaskId[],
): Promise<TabDeletion> {
  return invoke<TabDeletion>("restore_deleted_tab", { id, taskIds });
}
