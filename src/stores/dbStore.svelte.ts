import * as dbService from '$services/dbService';
import type { DbStats } from '$services/dbService';

class DbStore {
  stats = $state<DbStats | null>(null);
  loading = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);
  lastVacuumSaved = $state<number | null>(null);

  async refresh(): Promise<void> {
    this.loading = true;
    try {
      this.stats = await dbService.getDbStats();
      this.error = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
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