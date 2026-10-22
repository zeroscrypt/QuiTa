import * as dbService from '$services/dbService';
import type { BackupFile, DbStats } from '$services/dbService';

class DbStore {
  stats = $state<DbStats | null>(null);
  loading = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);
  lastVacuumSaved = $state<number | null>(null);
  /** Stored copies, newest first. */
  backups = $state<BackupFile[]>([]);
  backupBusy = $state(false);
  backupError = $state<string | null>(null);
  /** Flips after a copy lands, cleared at the start of the next one. */
  backupDone = $state(false);

  async refresh(): Promise<void> {
    this.loading = true;
    try {
      const [stats, info] = await Promise.all([
        dbService.getDbStats(),
        // A listing that fails must not blank the database statistics over
        // it: the copy folder is disposable, the numbers are not.
        dbService.backupInfo().catch(() => null),
      ]);
      this.stats = stats;
      if (info) this.backups = info.files;
      this.error = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }

  /**
   * Takes a copy right now and brings the listing back after the rotation.
   * A manual copy works with the feature switched off: the store is in the
   * settings, the button does what it says.
   */
  async backup(): Promise<void> {
    if (this.backupBusy) return;
    this.backupBusy = true;
    this.backupDone = false;
    try {
      const info = await dbService.runBackup();
      this.backups = info.files;
      this.backupError = null;
      this.backupDone = true;
    } catch (e) {
      this.backupError = e instanceof Error ? e.message : String(e);
    } finally {
      this.backupBusy = false;
    }
  }

  /** Compacts the file; returns the number of bytes actually reclaimed. */
  async vacuum(): Promise<void> {
    if (this.busy) return;
    const before = this.stats?.freelistCount ?? 0;
    this.busy = true;
    try {
      const after = await dbService.vacuumDb();
      this.lastVacuumSaved = Math.max(
        0,
        (this.stats?.sizeBytes ?? 0) - after.sizeBytes,
      );
      this.stats = after;
      // Nothing was on the free list, so there was nothing to reclaim.
      if (before === 0) this.lastVacuumSaved = 0;
      this.error = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.busy = false;
    }
  }

  /** Bytes currently held by the free list — what VACUUM could return. */
  get reclaimableBytes(): number {
    return (this.stats?.freelistCount ?? 0) * (this.stats?.pageSize ?? 0);
  }
}

export const dbStore = new DbStore();