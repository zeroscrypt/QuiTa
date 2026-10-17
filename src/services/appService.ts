import { invoke } from '@tauri-apps/api/core';

export interface AppInfo {
  name: string;
  version: string;
  descriptionRu: string;
  descriptionEn: string;
  author: string;
  license: string;
  repository: string;
  /** Global shortcut as registered on this platform. */
  shortcut: string;
  /**
   * Operating system this process runs on: "macos" | "windows" | "linux".
   * Taken from the backend because the settings list every shortcut, and a Mac
   * keyboard and a PC one spell the same combination differently.
   */
  platform: string;
  /** Field lengths the database accepts, in characters. */
  limits: Limits;
}

export interface Limits {
  title: number;
  tabName: number;
  comment: number;
}

/** Always up-to-date product metadata (version comes from Cargo). */
export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>('get_app_info');
}