import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Tauri APIs are unavailable outside the app shell; stores are imported for real
// so that regressions like runes in a non-`.svelte.ts` module are caught here.
const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

const TAB_ID = "tab-2";

function makeTab(overrides: Record<string, unknown> = {}) {
  return {
    id: TAB_ID,
    name: "Работа",
    color: "#4a90d9",
    sortOrder: 1,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    deletedAt: null,
    ...overrides,
  };
}

function makeDeletion() {
  return {
    tab: makeTab({ deletedAt: "2026-01-02T00:00:00Z" }),
    taskIds: ["task-1", "task-2"],
  };
}

/** `delete_tab` returns what the undo needs; everything else answers empty. */
function answerDelete(deletion = makeDeletion()) {
  invoke.mockImplementation(async (name: string) => {
    if (name === "delete_tab") return deletion;
    if (name === "restore_deleted_tab") return { tab: makeTab(), taskIds: [] };
    if (name === "list_tabs") return [makeTab({ id: "tab-1", sortOrder: 0 }), makeTab()];
    return [];
  });
}

describe("deleting a tab", () => {
  beforeEach(() => {
    invoke.mockReset();
    vi.useFakeTimers();
    // The undo path selects a tab, and selecting writes the choice to
    // localStorage. There is none under node, and `undo` reports the failure
    // rather than throwing — so without this the test would quietly measure
    // that the window refuses to undo, not that it undoes.
    const store = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => void store.set(key, value),
      removeItem: (key: string) => void store.delete(key),
      clear: () => store.clear(),
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("hides the tab and leaves its tasks alone until the window closes", async () => {
    const { tabStore } = await import("../src/stores/tabStore.svelte");
    tabStore.tabs = [makeTab({ id: "tab-1", sortOrder: 0 }), makeTab()];
    tabStore.activeTabId = "tab-1";
    answerDelete();

    const deletion = await tabStore.remove(TAB_ID);

    expect(invoke).toHaveBeenCalledWith("delete_tab", { id: TAB_ID });
    expect(
      invoke.mock.calls.some(([name]) => name === "finalize_tab_deletion"),
    ).toBe(false);
    expect(deletion?.taskIds).toEqual(["task-1", "task-2"]);
    expect(tabStore.tabs.map((t) => t.id)).toEqual(["tab-1"]);
  });

  it("refuses to remove the last tab without asking the backend", async () => {
    const { tabStore } = await import("../src/stores/tabStore.svelte");
    tabStore.tabs = [makeTab({ id: "tab-1" })];
    invoke.mockReset();

    const deletion = await tabStore.remove("tab-1");

    expect(deletion).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("sends the tasks to the trash when the undo window runs out", async () => {
    const { tabStore } = await import("../src/stores/tabStore.svelte");
    const { uiStore } = await import("../src/stores/uiStore.svelte");
    tabStore.tabs = [makeTab({ id: "tab-1", sortOrder: 0 }), makeTab()];
    answerDelete();

    uiStore.announceTabDeletion(makeDeletion());
    expect(invoke).not.toHaveBeenCalledWith("finalize_tab_deletion", {
      id: TAB_ID,
    });

    // Ten seconds of the countdown, one tick at a time.
    for (let second = 0; second < 10; second += 1) {
      await vi.advanceTimersByTimeAsync(1000);
    }

    expect(invoke).toHaveBeenCalledWith("finalize_tab_deletion", { id: TAB_ID });
    expect(uiStore.undoNotice).toBeNull();
  });

  it("sends nothing to the trash when the user undoes inside the window", async () => {
    const { tabStore } = await import("../src/stores/tabStore.svelte");
    const { uiStore } = await import("../src/stores/uiStore.svelte");
    tabStore.tabs = [makeTab({ id: "tab-1", sortOrder: 0 }), makeTab()];
    answerDelete();

    uiStore.announceTabDeletion(makeDeletion());
    await vi.advanceTimersByTimeAsync(3000);
    expect(await uiStore.undo()).toBe(true);

    // Past the point where the window would have closed: the undo won, so the
    // second half of the deletion must never run.
    for (let second = 0; second < 12; second += 1) {
      await vi.advanceTimersByTimeAsync(1000);
    }
    expect(
      invoke.mock.calls.some(([name]) => name === "finalize_tab_deletion"),
    ).toBe(false);
  });
});

describe("starting up", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("finishes a tab deletion the app closed in the middle of", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    invoke.mockReset();
    invoke.mockResolvedValueOnce([]); // purge_completed_tasks
    invoke.mockResolvedValueOnce([]); // purge_trash_by_age
    invoke.mockResolvedValueOnce([]); // sweep_pending_tab_deletions
    invoke.mockResolvedValueOnce([]); // list_tasks

    await taskStore.init();

    // Without this a tab hidden ten seconds before quitting would stay hidden
    // for ever, with its tasks still in place behind it.
    expect(invoke).toHaveBeenNthCalledWith(3, "sweep_pending_tab_deletions");
  });
});