import { invoke } from '@tauri-apps/api/core';

export interface AppInfo {
  name: string;
  version: string;
  descriptionRu: string;
  descriptionEn: string;
  author: string;
  license: string;
  repository: string;
}

/** Always up-to-date product metadata (version comes from Cargo). */
export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>('get_app_info');
}