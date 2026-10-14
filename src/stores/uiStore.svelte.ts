import type { ReparentReceipt, Task, TaskId } from '$lib/types';
import type { ImportSummary } from '$services/exportService';
import { t } from '$i18n';
import * as taskService from '$services/taskService';
import { taskStore } from './taskStore.svelte';
import { tabStore } from './tabStore.svelte';
import { settingsStore } from './settingsStore.svelte';

interface ConfirmDeleteState {
  task: Task;
  /** Number of subtasks that will be deleted together with the parent. */
  cascadeCount: number;
}

interface PendingUndo {
  taskId: TaskId;
  parentId: TaskId;
  /** Seconds left before the undo is gone for good. */
  secondsLeft: number;
  totalSeconds: number;
  receipt: ReparentReceipt;
  timer: ReturnType<typeof setInterval> | null;
}

const UNDO_SECONDS = 10;

class UiStore {
  settingsOpen = $state(false);
  aboutOpen = $state(false);
  confirmDelete = $state<ConfirmDeleteState | null>(null);
  exportMessage = $state<string | null>(null);
  exportError = $state<string | null>(null);

  /** Message with a countdown and an undo action, shown above the tabs. */
  undoNotice = $state<{
    taskId: TaskId;
    secondsLeft: number;
    totalSeconds: number;
    flattenedCount: number;
  } | null>(null);

  /** Incremented by the header button; App.svelte performs the scroll. */
  scrollToNewestRequest = $state(0);

  private unlistenAbout: (() => void) | null = null;
  private exportTimer: ReturnType<typeof setTimeout> | null = null;
  private pendingUndo: PendingUndo | null = null;

  /**
   * Announces a flattening move and starts the undo window.
   *
   * The countdown is the deadline: when it reaches zero the receipt is dropped
   * and the structure becomes permanent. Only one notice exists at a time —
   * a second move replaces the first and its undo.
   */
  announceFlatten(receipt: ReparentReceipt, parentId: TaskId): void {
    this.clearUndo();
    this.pendingUndo = {
      taskId: receipt.taskId,
      parentId,
      secondsLeft: UNDO_SECONDS,
      totalSeconds: UNDO_SECONDS,
      receipt,
      timer: null,
    };
    this.undoNotice = {
      taskId: receipt.taskId,
      secondsLeft: UNDO_SECONDS,
      totalSeconds: UNDO_SECONDS,
      flattenedCount: receipt.flattened.length,
    };
    this.pendingUndo.timer = setInterval(() => {
      const pending = this.pendingUndo;
      if (!pending) return;
      pending.secondsLeft -= 1;
      if (pending.secondsLeft <= 0) {
        this.clearUndo();
        return;
      }
      this.undoNotice = {
        taskId: pending.taskId,
        secondsLeft: pending.secondsLeft,
        totalSeconds: pending.totalSeconds,
        flattenedCount: pending.receipt.flattened.length,
      };
    }, 1000);
  }

  /** Restores the structure. Safe to call twice: the receipt is consumed. */
  async undoLastFlatten(): Promise<boolean> {
    const pending = this.pendingUndo;
    if (!pending) return false;
    this.clearUndo();
    await taskService.undoNest(pending.receipt);
    await taskStore.load();
    return true;
  }

  private clearUndo(): void {
    if (this.pendingUndo?.timer !== null && this.pendingUndo?.timer !== undefined) {
      clearInterval(this.pendingUndo.timer);
    }
    this.pendingUndo = null;
    this.undoNotice = null;
  }

  async init(): Promise<void> {
    const { listen } = await import('@tauri-apps/api/event');
    this.unlistenAbout?.();
    this.unlistenAbout = await listen('quita:about', () => {
      this.aboutOpen = true;
    });
  }

  /** Incremented by the header button; App.svelte performs the scroll. */
  requestScrollToNewest(): void {
    this.scrollToNewestRequest++;
  }

  /** Picks a JSON file and merges it into the database. */
  async runImport(): Promise<void> {
    this.exportError = null;
    this.exportMessage = null;
    try {
      const { importTasks } = await import('$services/exportService');
      const summary = await importTasks();
      if (!summary) return;
      this.importSummary = summary;
      if (this.exportTimer !== null) clearTimeout(this.exportTimer);
      this.exportTimer = setTimeout(() => {
        this.importSummary = null;
        this.exportTimer = null;
      }, 6000);
      // The file can carry tabs and settings, so everything is reloaded —
      // otherwise a restored tab or a switched theme would stay invisible.
      await tabStore.reload();
      await settingsStore.init();
      await taskStore.load();
      await taskStore.loadDeleted();
      await tabStore.refreshStats();
    } catch (e) {
      this.exportError = e instanceof Error ? e.message : String(e);
    }
  }

  askDelete(task: Task, cascadeCount: number): void {
    this.confirmDelete = { task, cascadeCount };
  }

  cancelDelete(): void {
    this.confirmDelete = null;
  }

  /** Progress of an export/import round trip, shown as a toast. */
  importSummary = $state<ImportSummary | null>(null);

  async runExport(): Promise<void> {
    this.exportError = null;
    try {
      const { exportTasks } = await import('$services/exportService');
      const path = await exportTasks();
      this.exportMessage = path === null ? null : t('settings.exportDone', { path });
      if (path === null) return;
      // One timer at a time: a second export must not be hidden early.
      if (this.exportTimer !== null) clearTimeout(this.exportTimer);
      this.exportTimer = setTimeout(() => {
        this.exportMessage = null;
        this.exportTimer = null;
      }, 5000);
    } catch (e) {
      this.exportMessage = null;
      this.exportError = e instanceof Error ? e.message : String(e);
    }
  }
}
export const uiStore = new UiStore();
