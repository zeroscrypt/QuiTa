import { invoke } from '@tauri-apps/api/core';

export interface DbStats {
  /** On-disk size of the database file plus its `-wal`/`-shm` companions. */
  sizeBytes: number;
  pageSize: number;
  pageCount: number;
  /** Pages freed by deletions and reusable for new rows. */
  freelistCount: number;
  taskCount: number;
  activeCount: number;
  deletedCount: number;
  tabCount: number;
}

/** One stored copy of the database. */
export interface BackupFile {
  /** `quita-backup-20261022-103500.db` */
  name: string;
  /** Rebuilt from the stamp in the name, e.g. "2026-10-22 10:35:00". */
  createdAt: string;
  sizeBytes: number;
}

/** The stored copies, newest first — already rotated. */
export interface BackupInfo {
  files: BackupFile[];
}

export function getDbStats(): Promise<DbStats> {
  return invoke<DbStats>('db_stats');
}

/** Compacts the database file and returns the fresh statistics. */
export function vacuumDb(): Promise<DbStats> {
  return invoke<DbStats>('vacuum_db');
}

/** The stored copies, newest first. */
export function backupInfo(): Promise<BackupInfo> {
  return invoke<BackupInfo>('backup_info');
}

/** Takes a fresh copy right now and returns the listing after the rotation. */
export function runBackup(): Promise<BackupInfo> {
  return invoke<BackupInfo>('backup_database');
}