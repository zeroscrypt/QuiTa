import { invoke } from '@tauri-apps/api/core';
import type { Tab, TaskId } from '$lib/types';

export function listTabs(): Promise<Tab[]> {
  return invoke<Tab[]>('list_tabs');
}

export function createTab(name?: string): Promise<Tab> {
  return invoke<Tab>('create_tab', { name: name ?? null });
}

export function renameTab(id: TaskId, name: string): Promise<Tab> {
  return invoke<Tab>('rename_tab', { id, name });
}

export function reorderTabs(orderedIds: TaskId[]): Promise<Tab[]> {
  return invoke<Tab[]>('reorder_tabs', { orderedIds });
}

/** Done/total counters per tab. Subtasks count as tasks. */
export interface TabStat {
  tabId: TaskId;
  total: number;
  done: number;
}

export function tabStats(): Promise<TabStat[]> {
  return invoke<TabStat[]>('tab_stats');
}

/** Deletes a tab; its tasks are moved to `fallbackTabId` so nothing is lost. */
export function deleteTab(id: TaskId, fallbackTabId: TaskId | null): Promise<unknown[]> {
  return invoke<unknown[]>('delete_tab', { id, fallbackTabId });
}