import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '$lib/types';

export function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('get_settings');
}

export function updateSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>('update_settings', { settings });
}
