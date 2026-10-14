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

export function getDbStats(): Promise<DbStats> {
  return invoke<DbStats>('db_stats');
}

/** Compacts the database file and returns the fresh statistics. */
export function vacuumDb(): Promise<DbStats> {
  return invoke<DbStats>('vacuum_db');
}