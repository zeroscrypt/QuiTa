import { invoke } from '@tauri-apps/api/core';

export interface ImportSummary {
  path: string;
  format: string;
  added: number;
  updated: number;
  skipped: number;
  tabsAdded: number;
  tabsUpdated: number;
  settingsRestored: boolean;
}

/**
 * Opens the native save dialog and writes the whole application state as
 * JSON: tasks with their hierarchy, the trash, the tabs and the settings.
 * Resolves to the written path, or null when the user cancelled.
 */
export function exportTasks(): Promise<string | null> {
  return invoke<string | null>('export_tasks');
}

/**
 * Opens a JSON file and merges it into the database: tabs first, then tasks,
 * then the settings. Nothing is deleted.
 */
export function importTasks(): Promise<ImportSummary | null> {
  return invoke<ImportSummary | null>('import_tasks');
}