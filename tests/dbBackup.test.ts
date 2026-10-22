import { beforeEach, describe, expect, it, vi } from "vitest";

// Tauri APIs are unavailable outside the app shell; the store is imported for
// real so that regressions like runes in a non-`.svelte.ts` module are caught.
const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

const copy1 = {
  name: "quita-backup-20261022-103500.db",
  createdAt: "2026-10-22 10:35:00",
  sizeBytes: 20480,
};
const copy2 = {
  name: "quita-backup-20261021-090000.db",
  createdAt: "2026-10-21 09:00:00",
  sizeBytes: 10240,
};

describe("dbStore backups", () => {
  beforeEach(async () => {
    invoke.mockReset();
    const { dbStore } = await import("../src/stores/dbStore.svelte");
    dbStore.stats = null;
    dbStore.backups = [];
    dbStore.backupBusy = false;
    dbStore.backupDone = false;
    dbStore.backupError = null;
    dbStore.error = null;
  });

  it("backs up on demand and refreshes the listing with the rotation in it", async () => {
    invoke.mockResolvedValueOnce({ files: [copy1, copy2] });
    const { dbStore } = await import("../src/stores/dbStore.svelte");
    await dbStore.backup();
    expect(invoke).toHaveBeenCalledWith("backup_database");
    expect(dbStore.backups).toEqual([copy1, copy2]);
    expect(dbStore.backupDone).toBe(true);
    expect(dbStore.backupBusy).toBe(false);
    expect(dbStore.backupError).toBeNull();
  });

  it("reports a failed copy without claiming success", async () => {
    invoke.mockRejectedValueOnce(new Error("disk full"));
    const { dbStore } = await import("../src/stores/dbStore.svelte");
    await dbStore.backup();
    expect(dbStore.backupDone).toBe(false);
    expect(dbStore.backupError).toBe("disk full");
    expect(dbStore.backups).toEqual([]);
  });
});
