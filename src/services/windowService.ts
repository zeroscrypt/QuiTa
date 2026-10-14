import { invoke } from '@tauri-apps/api/core';

/** Toggles the window between its default size and the full monitor height. */
export function toggleWindowHeight(): Promise<void> {
  return invoke<void>('toggle_window_height');
}