import type {
  ReparentReceipt,
  Tab,
  TabDeletion,
  Task,
  TaskId,
} from "$lib/types";
import type { ImportSummary } from "$services/exportService";
import { t } from "$i18n";
import * as taskService from "$services/taskService";
import { taskStore } from "./taskStore.svelte";
import { tabStore } from "./tabStore.svelte";
import { settingsStore } from "./settingsStore.svelte";

interface ConfirmDeleteState {
  task: Task;
  /** Number of subtasks that will be deleted together with the parent. */
  cascadeCount: number;
}

interface ConfirmTabDeleteState {
  tab: Tab;
  /** Tasks that will be moved to the trash together with the tab. */
  taskCount: number;
}

/**
 * One undo window for two kinds of undoable move. Keeping them in one notice
 * is deliberate: two notices could be on screen at once, and the second would
 * silently take the first one's place.
 */
type UndoNotice =
  | {
      kind: "flatten";
      /** Seconds left before the undo is gone for good. */
      secondsLeft: number;
      totalSeconds: number;
      flattenedCount: number;
    }
  | {
      kind: "tab";
      secondsLeft: number;
      totalSeconds: number;
      taskCount: number;
    };

type PendingUndo =
  | {
      kind: "flatten";
      parentId: TaskId;
      secondsLeft: number;
      totalSeconds: number;
      receipt: ReparentReceipt;
      timer: ReturnType<typeof setInterval> | null;
    }
  | {
      kind: "tab";
      secondsLeft: number;
      totalSeconds: number;
      deletion: TabDeletion;
      timer: ReturnType<typeof setInterval> | null;
    };

const UNDO_SECONDS = 10;

class UiStore {
  settingsOpen = $state(false);
  aboutOpen = $state(false);
  confirmDelete = $state<ConfirmDeleteState | null>(null);
  confirmTabDelete = $state<ConfirmTabDeleteState | null>(null);
  exportMessage = $state<string | null>(null);
  exportError = $state<string | null>(null);

  /** Message with a countdown and an undo action, shown above the tabs. */
  undoNotice = $state<UndoNotice | null>(null);

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
    this.startUndo({
      kind: "flatten",
      parentId,
      secondsLeft: UNDO_SECONDS,
      totalSeconds: UNDO_SECONDS,
      receipt,
      timer: null,
    });
  }

  /**
   * Announces a tab deletion and starts its undo window.
   *
   * The tab comes back with exactly the tasks it took with it, so nothing has
   * to be reconstructed on the way out.
   */
  announceTabDeletion(deletion: TabDeletion): void {
    this.startUndo({
      kind: "tab",
      secondsLeft: UNDO_SECONDS,
      totalSeconds: UNDO_SECONDS,
      deletion,
      timer: null,
    });
  }

  /**
   * Starts the countdown and publishes the notice.
   *
   * The countdown is the deadline: when it reaches zero the receipt is dropped
   * and the move becomes permanent. Only one notice exists at a time — a second
   * move replaces the first and its undo.
   */
  private startUndo(pending: PendingUndo): void {
    this.clearUndo();
    this.pendingUndo = pending;
    this.publishUndo(pending.secondsLeft);

    pending.timer = setInterval(() => {
      const current = this.pendingUndo;
      if (!current) return;
      current.secondsLeft -= 1;
      if (current.secondsLeft <= 0) {
        this.clearUndo();
        return;
      }
      this.publishUndo(current.secondsLeft);
    }, 1000);
  }

  private publishUndo(secondsLeft: number): void {
    const pending = this.pendingUndo;
    if (!pending) return;
    this.undoNotice =
      pending.kind === "flatten"
        ? {
            kind: "flatten",
            secondsLeft,
            totalSeconds: pending.totalSeconds,
            flattenedCount: pending.receipt.flattened.length,
          }
        : {
            kind: "tab",
            secondsLeft,
            totalSeconds: pending.totalSeconds,
            taskCount: pending.deletion.taskIds.length,
          };
  }

  /** Takes back whichever move the notice is about. Safe to call twice. */
  async undo(): Promise<boolean> {
    const pending = this.pendingUndo;
    if (!pending) return false;
    // Cleared only once the undo actually went through. Doing it first meant a
    // failed call left the user with no way back and no explanation: the notice
    // was gone and the nesting had become permanent.
    try {
      if (pending.kind === "flatten") {
        await taskService.undoNest(pending.receipt);
      } else {
        await tabStore.restore(pending.deletion);
      }
    } catch (e) {
      taskStore.showError(e);
      return false;
    }
    this.clearUndo();
    await taskStore.load();
    return true;
  }

  /** Opens the confirmation for deleting a tab. */
  askDeleteTab(tab: Tab, taskCount: number): void {
    this.confirmTabDelete = { tab, taskCount };
  }

  cancelTabDelete(): void {
    this.confirmTabDelete = null;
  }

  private clearUndo(): void {
    if (
      this.pendingUndo?.timer !== null &&
      this.pendingUndo?.timer !== undefined
    ) {
      clearInterval(this.pendingUndo.timer);
    }
    this.pendingUndo = null;
    this.undoNotice = null;
  }

  async init(): Promise<void> {
    const { listen } = await import("@tauri-apps/api/event");
    this.unlistenAbout?.();
    this.unlistenAbout = await listen("quita:about", () => {
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
      const { importTasks } = await import("$services/exportService");
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
      const { exportTasks } = await import("$services/exportService");
      const path = await exportTasks();
      this.exportMessage =
        path === null ? null : t("settings.exportDone", { path });
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
