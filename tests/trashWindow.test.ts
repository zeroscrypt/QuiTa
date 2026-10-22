import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}));

const { taskStore } = await import('../src/stores/taskStore.svelte');

function task(id: string) {
  return {
    id,
    parentId: null,
    tabId: null,
    title: id,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    completedAt: null,
    deletedAt: '2026-01-02T00:00:00Z',
    dueDate: null,
    comment: null,
    sortOrder: 0,
    completionEventId: null,
    autoCompletedBy: null,
    notRelevantAt: null,
    notRelevantEventId: null,
    autoNotRelevantBy: null,
  };
}

function page(from: number, count: number, total: number) {
  return { tasks: Array.from({ length: count }, (_, i) => task(`t${from + i}`)), total };
}

describe('the trash window', () => {
  beforeEach(() => {
    invoke.mockReset();
    taskStore.deleted = [];
    taskStore.deletedTotal = 0;
  });

  it('reports a total that is larger than the window', async () => {
    invoke.mockResolvedValueOnce(page(0, 200, 328));
    await taskStore.loadDeleted();

    expect(taskStore.deleted).toHaveLength(200);
    expect(taskStore.deletedTotal).toBe(328);
    expect(taskStore.hasMoreDeleted).toBe(true);
  });

  it('asks for the next window at the offset it already has', async () => {
    invoke.mockResolvedValueOnce(page(0, 200, 328));
    await taskStore.loadDeleted();
    invoke.mockResolvedValueOnce(page(200, 128, 328));
    await taskStore.loadMoreDeleted();

    expect(invoke).toHaveBeenLastCalledWith('list_deleted_tasks', { offset: 200 });
    expect(taskStore.deleted).toHaveLength(328);
    expect(taskStore.hasMoreDeleted).toBe(false);
  });

  it('does not list the same task twice when a window shifts', async () => {
    // Restoring a task in another view shifts every row, so the next window can
    // repeat one the list already has. The rows are keyed by id, and a repeat
    // would throw.
    invoke.mockResolvedValueOnce(page(0, 2, 4));
    await taskStore.loadDeleted();
    invoke.mockResolvedValueOnce({ tasks: [task('t1'), task('t2'), task('t3')], total: 3 });
    await taskStore.loadMoreDeleted();

    const ids = taskStore.deleted.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toEqual(['t0', 't1', 't2', 't3']);
  });

  it('stops asking once everything is loaded', async () => {
    invoke.mockResolvedValueOnce(page(0, 2, 2));
    await taskStore.loadDeleted();
    expect(taskStore.hasMoreDeleted).toBe(false);

    invoke.mockClear();
    await taskStore.loadMoreDeleted();
    expect(invoke).not.toHaveBeenCalled();
  });

  it('releases the busy flag when a window fails to load', async () => {
    invoke.mockResolvedValueOnce(page(0, 2, 4));
    await taskStore.loadDeleted();
    invoke.mockRejectedValueOnce(new Error('database is locked'));

    await taskStore.loadMoreDeleted();

    // Otherwise the button would stay disabled forever with no way to retry.
    expect(taskStore.loadingMoreDeleted).toBe(false);
  });
});