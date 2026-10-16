import { beforeEach, describe, expect, it, vi } from "vitest";

// Tauri APIs are unavailable outside the app shell; stores are imported for real
// so that regressions like runes in a non-`.svelte.ts` module are caught here.
const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

function makeTask(overrides: Record<string, unknown> = {}) {
  return {
    id: "id-1",
    parentId: null,
    tabId: null,
    title: "Task",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    completedAt: null,
    deletedAt: null,
    dueDate: null,
    comment: null,
    sortOrder: -1,
    completionEventId: null,
    autoCompletedBy: null,
    ...overrides,
  };
}

/**
 * What every move into the trash answers: what arrived, and what it pushed out
 * of the trash to stay within its bound.
 */
function sweep(moved: unknown[] = [], evicted: unknown[] = []) {
  return { deleted: moved, evicted };
}

describe("taskStore", () => {
  beforeEach(async () => {
    invoke.mockReset();
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    taskStore.nodes = [];
    taskStore.error = null;
    taskStore.notice = null;
  });

  it("adds a new root task to the top of the list", async () => {
    invoke.mockResolvedValueOnce(
      makeTask({ id: "new", title: "Новая задача" }),
    );
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    const { tabStore } = await import("../src/stores/tabStore.svelte");
    tabStore.tabs = [
      {
        id: "tab-1",
        name: "Tab",
        color: "#4c6fff",
        sortOrder: 0,
        createdAt: "",
        updatedAt: "",
        deletedAt: null,
      },
    ];
    tabStore.activeTabId = "tab-1";

    await taskStore.addRoot("Новая задача");

    expect(invoke).toHaveBeenCalledWith("create_task", {
      title: "Новая задача",
      parentId: null,
      tabId: "tab-1",
    });
    expect(taskStore.nodes.map((n) => n.task.title)).toEqual(["Новая задача"]);
  });

  it("adds a subtask to the matching parent", async () => {
    invoke.mockResolvedValueOnce(makeTask({ id: "parent" }));
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    taskStore.nodes = [{ task: makeTask({ id: "parent" }), children: [] }];
    invoke.mockResolvedValueOnce(makeTask({ id: "child", parentId: "parent" }));

    await taskStore.addSubtask("parent", "Подзадача");

    expect(invoke).toHaveBeenCalledWith("create_task", {
      title: "Подзадача",
      parentId: "parent",
      tabId: null,
    });
    expect(taskStore.nodes[0].children).toHaveLength(1);
  });

  it("removes the node after soft delete", async () => {
    invoke.mockResolvedValueOnce({
      deleted: [makeTask({ id: "a" })],
      evicted: [],
    });
    invoke.mockResolvedValueOnce({ tasks: [], total: 0 });
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    taskStore.nodes = [
      { task: makeTask({ id: "a" }), children: [] },
      { task: makeTask({ id: "b" }), children: [] },
    ];

    await taskStore.remove("a");

    expect(taskStore.nodes.map((n) => n.task.id)).toEqual(["b"]);
    expect(taskStore.notice).toBeNull();
  });

  // The size rule is the one deletion nobody asked for: the task the user
  // deleted is safe in the trash, and a different one is gone. Staying silent
  // would leave a task missing with no account of where it went.
  it("names the task the full trash pushed out", async () => {
    invoke.mockResolvedValueOnce({
      deleted: [makeTask({ id: "a", title: "Только что удалённая" })],
      evicted: [makeTask({ id: "z", title: "Самая старая" })],
    });
    invoke.mockResolvedValueOnce({ tasks: [], total: 0 });
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    taskStore.nodes = [{ task: makeTask({ id: "a" }), children: [] }];

    await taskStore.remove("a");

    expect(taskStore.notice).toContain("Самая старая");
    expect(taskStore.notice).not.toContain("Только что удалённая");
  });

  it("toggle applies the outcome in place instead of reloading the list", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    const { settingsStore } =
      await import("../src/stores/settingsStore.svelte");
    taskStore.nodes = [
      { task: makeTask({ id: "a" }), children: [] },
      { task: makeTask({ id: "b" }), children: [] },
    ];
    invoke.mockResolvedValueOnce({
      updated: [makeTask({ id: "b", completedAt: "2026-01-02T00:00:00Z" })],
    });

    await taskStore.toggle("b");

    expect(invoke).toHaveBeenNthCalledWith(1, "toggle_task_completion", {
      id: "b",
      cascadeComplete: settingsStore.cascadeComplete,
    });
    // No full reload: the DOM (and the scroll position) stays untouched. The
    // only other call is the tab counter refresh, which is not a list reload.
    const commands = invoke.mock.calls.map((call) => call[0]);
    expect(commands.filter((name) => name === "list_tasks")).toHaveLength(0);
    expect(commands).toContain("tab_stats");
    expect(taskStore.nodes.map((n) => n.task.id)).toEqual(["a"]);
  });

  it("brings a reopened parent back into the list with its subtasks", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    taskStore.nodes = [
      { task: makeTask({ id: "child", parentId: "parent" }), children: [] },
    ];
    invoke.mockResolvedValueOnce({
      updated: [
        makeTask({ id: "parent" }),
        makeTask({ id: "child", parentId: "parent" }),
      ],
    });
    invoke.mockResolvedValueOnce([makeTask({ id: "sub", parentId: "parent" })]);

    await taskStore.toggle("child");

    expect(invoke).toHaveBeenNthCalledWith(2, "list_subtasks", {
      parentId: "parent",
    });
    const parent = taskStore.nodes.find((n) => n.task.id === "parent");
    expect(parent?.children.map((c) => c.id)).toEqual(["sub"]);
  });

  it("reparent moves a task under another parent", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    invoke.mockResolvedValueOnce(makeTask({ id: "a", parentId: "p" })); // reparent_task
    invoke.mockResolvedValueOnce([]); // list_tasks

    await taskStore.reparent("a", "p");

    expect(invoke).toHaveBeenNthCalledWith(1, "reparent_task", {
      id: "a",
      parentId: "p",
      placeAtTop: true,
    });
  });

  it("moveToTab sends the task to another tab", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    invoke.mockResolvedValueOnce(makeTask({ id: "a", tabId: "tab-2" }));
    invoke.mockResolvedValueOnce([]);

    await taskStore.moveToTab("a", "tab-2");

    expect(invoke).toHaveBeenNthCalledWith(1, "move_task_to_tab", {
      id: "a",
      tabId: "tab-2",
    });
  });

  it("purges expired completed tasks when the retention setting is on", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    const { settingsStore } =
      await import("../src/stores/settingsStore.svelte");
    settingsStore.autoDeleteDays = 30;
    invoke.mockReset();
    invoke.mockResolvedValueOnce(sweep([makeTask({ id: "gone" })])); // purge_completed_tasks
    invoke.mockResolvedValueOnce([]); // purge_trash_by_age
    invoke.mockResolvedValueOnce(sweep()); // sweep_pending_tab_deletions
    invoke.mockResolvedValueOnce([]); // list_tasks

    await taskStore.init();

    expect(invoke).toHaveBeenNthCalledWith(1, "purge_completed_tasks");
    settingsStore.autoDeleteDays = null;
  });

  it("sweeps the trash by age on every start, setting or no setting", async () => {
    // The command reads the setting itself and does nothing while it is empty,
    // so the call is unconditional: a trash that only ages out after the user
    // happens to touch that field would be a trash that never empties.
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    const { settingsStore } =
      await import("../src/stores/settingsStore.svelte");
    settingsStore.trashRetentionDays = null;
    invoke.mockReset();
    invoke.mockResolvedValueOnce(sweep()); // purge_completed_tasks
    invoke.mockResolvedValueOnce([makeTask({ id: "forgotten" })]); // purge_trash_by_age
    invoke.mockResolvedValueOnce(sweep()); // sweep_pending_tab_deletions
    invoke.mockResolvedValueOnce([]); // list_tasks

    await taskStore.init();

    expect(invoke).toHaveBeenNthCalledWith(2, "purge_trash_by_age");
    settingsStore.trashRetentionDays = null;
  });

  it("names what an automatic move pushed out of the trash", async () => {
    // The completion timer and the tab sweep can fill the bin past its bound,
    // and whatever they push out is gone for good. Silence here would leave the
    // bound a rule that only holds when nothing else happens.
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    invoke.mockReset();
    // The notice names the task, so the title is what the test watches for.
    invoke.mockResolvedValueOnce(
      sweep([], [makeTask({ id: "pushed-out", title: "Вытесненная" })]),
    );
    invoke.mockResolvedValueOnce([]); // purge_trash_by_age
    invoke.mockResolvedValueOnce(sweep()); // sweep_pending_tab_deletions
    invoke.mockResolvedValueOnce([]); // list_tasks

    await taskStore.init();

    expect(taskStore.notice).toContain("Вытесненная");
  });

  it("says nothing when no automatic move pushed anything out", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    invoke.mockReset();
    invoke.mockResolvedValueOnce(sweep()); // purge_completed_tasks
    invoke.mockResolvedValueOnce([]); // purge_trash_by_age
    invoke.mockResolvedValueOnce(sweep()); // sweep_pending_tab_deletions
    invoke.mockResolvedValueOnce([]); // list_tasks

    await taskStore.init();

    expect(taskStore.notice).toBeNull();
  });

  it("passes the cascade setting from the store", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    const { settingsStore } =
      await import("../src/stores/settingsStore.svelte");
    settingsStore.cascadeComplete = false;
    invoke.mockResolvedValueOnce({ updated: [] });
    invoke.mockResolvedValueOnce([]);

    await taskStore.toggle("a");

    expect(invoke).toHaveBeenNthCalledWith(1, "toggle_task_completion", {
      id: "a",
      cascadeComplete: false,
    });
    settingsStore.cascadeComplete = true;
  });

  it("keeps completed tasks visible when the setting is on", async () => {
    const { taskStore } = await import("../src/stores/taskStore.svelte");
    const { settingsStore } =
      await import("../src/stores/settingsStore.svelte");
    settingsStore.showCompleted = true;

    invoke.mockResolvedValueOnce([
      {
        task: makeTask({ id: "a", completedAt: "2026-01-02T00:00:00Z" }),
        children: [],
      },
    ]);
    await taskStore.reload();

    expect(taskStore.nodes).toHaveLength(1);
    settingsStore.showCompleted = false;
  });
});
