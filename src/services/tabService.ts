import { invoke } from "@tauri-apps/api/core";
import type { Tab, TabDeletion, TaskId } from "$lib/types";

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

/** Deletes a tab; its tasks are moved to `fallbackTabId` so nothing is lost. */
export function deleteTab(id: TaskId): Promise<TabDeletion> {
  return invoke<TabDeletion>("delete_tab", { id });
}

export function restoreDeletedTab(
  id: TaskId,
  taskIds: TaskId[],
): Promise<TabDeletion> {
  return invoke<TabDeletion>("restore_deleted_tab", { id, taskIds });
}
