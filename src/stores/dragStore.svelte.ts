import type { Task, TaskId } from '$lib/types';
import { taskStore } from './taskStore.svelte';
import { tabStore } from './tabStore.svelte';

/** Where the pointer is inside a row: nest (middle) or reorder (edges). */
type Zone = 'nest' | 'insert' | 'none';

/**
 * Pointer-based drag & drop.
 *
 * HTML5 drag events do not fire reliably inside the WKWebView on macOS, so the
 * gesture is implemented with pointer events plus `elementFromPoint` hit
 * testing. Every row is split into three horizontal zones, so the result is
 * unambiguous and does not depend on timing:
 *
 *   ── top quarter ── insert before
 *   ── middle half  ─ drop as a subtask of this task
 *   ── bottom quarter ─ insert after
 *
 * Drop targets are marked with data attributes:
 *  - `data-drop-tab`    — a tab (moves the task and its subtasks there)
 *  - `data-drop-parent` — an active root task that may adopt the dragged task
 *  - `data-task-row`    — a row in the root list
 */
class DragStore {
  active = $state(false);
  title = $state('');
  pointer = $state({ x: 0, y: 0 });
  zone = $state<Zone>('none');

  hoverTabId = $state<TaskId | null>(null);
  hoverParentId = $state<TaskId | null>(null);
  dropIndex = $state<number | null>(null);
  /** Sibling group the insertion index refers to; null = the root list. */
  dropParentId = $state<TaskId | null>(null);

  private source: {
    id: TaskId;
    kind: 'root' | 'sub';
    title: string;
    family: Set<TaskId>;
  } | null = null;
  private armed = false;
  private startX = 0;
  private startY = 0;

  get sourceId(): TaskId | null {
    return this.source?.id ?? null;
  }

  get nesting(): boolean {
    return this.zone === 'nest' && this.hoverParentId !== null;
  }

  begin(event: PointerEvent, task: Task): void {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    // Buttons, inputs and menus keep their normal behaviour.
    if (target.closest('button, input, textarea, .menu, .switch')) return;

    const family = new Set<TaskId>([task.id]);
    for (const node of taskStore.nodes) {
      if (task.parentId === null && node.task.id === task.id) {
        for (const child of node.children) family.add(child.id);
      }
      if (task.parentId !== null && node.task.id === task.parentId) {
        family.add(node.task.id);
      }
    }

    this.source = {
      id: task.id,
      kind: task.parentId === null ? 'root' : 'sub',
      title: task.title,
      family,
    };
    this.armed = false;
    this.startX = event.clientX;
    this.startY = event.clientY;
  }

  move(event: PointerEvent): void {
    if (!this.source) return;
    // The button can be released outside the window, in which case no
    // `pointerup` arrives here. `buttons === 0` means it already is: abandon
    // the gesture instead of leaving a ghost stuck on screen and turning the
    // next click into an unintended drop.
    if (event.buttons === 0) {
      this.cancel();
      return;
    }
    if (!this.armed) {
      // A small threshold avoids starting a drag on a plain click.
      const dx = Math.abs(event.clientX - this.startX);
      const dy = Math.abs(event.clientY - this.startY);
      if (dx < 5 && dy < 5) return;
      this.armed = true;
      this.active = true;
      this.title = this.source.title;
    }
    event.preventDefault();
    this.pointer = { x: event.clientX, y: event.clientY };
    this.hitTest(event.clientX, event.clientY);
  }

  /**
   * Finds the row under the pointer and where an insertion would land.
   *
   * Resolved geometrically instead of by `closest()`: the insertion line is a
   * sibling element, so hovering the very line the user aims at used to resolve
   * to "no row at all" and the drop did nothing. Working from the rectangles
   * also turns the space below the last row into a real position.
   */
  private locate(rows: HTMLElement[], y: number): { index: number; rel: number } {
    if (rows.length === 0) return { index: 0, rel: 0 };
    for (let index = 0; index < rows.length; index += 1) {
      const rect = rows[index].getBoundingClientRect();
      if (y < rect.bottom) {
        // Clamped: a pointer above the row must read as "at its top", not as a
        // negative position the caller would have to know about.
        const rel = (y - rect.top) / Math.max(rect.height, 1);
        return { index, rel: Math.min(Math.max(rel, 0), 1) };
      }
    }
    // Below every row: append after the last one. This is the only way to put
    // a task at the very end, so the empty space under the list counts as it.
    return { index: rows.length, rel: 1 };
  }

  private hitTest(x: number, y: number): void {
    if (!this.source) return;
    const element = document.elementFromPoint(x, y) as HTMLElement | null;

    const tabEl = element?.closest<HTMLElement>('[data-drop-tab]');
    if (tabEl) {
      this.hoverTabId = tabEl.dataset.dropTab ?? null;
      this.hoverParentId = null;
      this.dropIndex = null;
      this.zone = 'none';
      return;
    }
    this.hoverTabId = null;

    // A subtask row belongs to a sibling group. Handling it before the root
    // row matters: the subtask list is rendered inside the parent row, so the
    // root row would otherwise swallow the gesture and nest the dragged task
    // under the wrong parent.
    const subParentId =
      element?.closest<HTMLElement>('[data-subtask-row]')?.dataset.subtaskParent ?? null;
    if (subParentId !== null) {
      const siblings = Array.from(
        document.querySelectorAll<HTMLElement>(
          `[data-subtask-parent="${CSS.escape(subParentId)}"]`,
        ),
      );
      const { index, rel } = this.locate(siblings, y);
      this.zone = 'insert';
      this.hoverParentId = null;
      this.dropParentId = subParentId;
      this.dropIndex = rel >= 0.5 ? index + 1 : index;
      return;
    }
    this.dropParentId = null;

    const rows = Array.from(document.querySelectorAll<HTMLElement>('[data-task-row]'));
    const { index, rel } = this.locate(rows, y);
    if (index >= rows.length) {
      // Past the last root row only "insert after it" is possible.
      this.zone = 'insert';
      this.hoverParentId = null;
      this.dropIndex = rows.length;
      return;
    }

    const parentId = rows[index].dataset.dropParent ?? null;
    const canNest =
      parentId !== null && !this.source.family.has(parentId) && parentId !== this.source.id;

    // The middle half of the row nests; the top/bottom quarters reorder.
    const middle = rel > 0.25 && rel < 0.75;
    if (middle && canNest) {
      this.zone = 'nest';
      this.hoverParentId = parentId;
      this.dropIndex = null;
      return;
    }

    this.zone = 'insert';
    this.hoverParentId = null;
    this.dropIndex = rel >= 0.75 ? index + 1 : index;
  }

  async drop(): Promise<void> {
    const source = this.source;
    const { hoverTabId, hoverParentId, dropIndex, dropParentId, zone } = this;
    this.reset();
    if (!source) return;

    if (hoverTabId && hoverTabId !== tabStore.activeTabId) {
      await taskStore.moveToTab(source.id, hoverTabId);
      return;
    }
    // Released over a subtask row: reorder within that sibling group, or join
    // it when the dragged task is not a subtask yet.
    if (dropParentId && dropIndex !== null) {
      const node = taskStore.nodes.find((n) => n.task.id === dropParentId);
      if (!node) return;
      const ids = node.children.map((child) => child.id);
      const from = ids.indexOf(source.id);
      if (from >= 0) {
        let to = dropIndex > from ? dropIndex - 1 : dropIndex;
        if (to === from) return;
        ids.splice(to, 0, ...ids.splice(from, 1));
      } else {
        // Joining a group from outside lands where the line is shown, which
        // may be the very end. A task that has subtasks is flattened: they
        // join the same group, so they are part of the order being written —
        // the move can be undone from the notice.
        const flattened = await taskStore.nestWithChildren(source.id, dropParentId);
        if (flattened === 0) {
          ids.splice(Math.min(dropIndex, ids.length), 0, source.id);
        } else {
          // `load()` has already placed the moved task and its former children
          // at the end; re-read the real group instead of guessing it.
          const fresh = taskStore.nodes.find((n) => n.task.id === dropParentId);
          if (!fresh) return;
          const group = fresh.children.map((child) => child.id);
          const at = group.indexOf(source.id);
          if (at < 0) return;
          group.splice(at, 1);
          group.splice(Math.min(dropIndex, group.length), 0, source.id);
          await taskStore.reorder(group);
          return;
        }
      }
      await taskStore.reorder(ids);
      return;
    }
    // Released over the middle of a task: it becomes a subtask of it, at the
    // end of that subtask list.
    if (zone === 'nest' && hoverParentId) {
      // A task with subtasks is flattened: two levels is the limit, so the
      // children become siblings. The notice offers an undo.
      await taskStore.nestWithChildren(source.id, hoverParentId);
      return;
    }
    if (dropIndex === null) return;

    const ids = taskStore.nodes.map((n) => n.task.id);
    const from = ids.indexOf(source.id);
    let to = dropIndex;
    if (from < 0) {
      // A subtask is not in the root list yet: promote it and place it there.
      await taskStore.reparent(source.id, null);
      ids.splice(Math.min(to, ids.length), 0, source.id);
      await taskStore.reorder(ids);
      return;
    }
    if (to > from) to -= 1;
    if (to === from) return;
    // `from >= 0` means the task is already a root task (subtasks are promoted
    // in the branch above), so a plain reorder is enough here.
    ids.splice(to, 0, ...ids.splice(from, 1));
    await taskStore.reorder(ids);
  }

  cancel(): void {
    this.reset();
  }

  private reset(): void {
    this.source = null;
    this.armed = false;
    this.active = false;
    this.hoverTabId = null;
    this.hoverParentId = null;
    this.dropIndex = null;
    this.dropParentId = null;
    this.zone = 'none';
  }
}

export const dragStore = new DragStore();