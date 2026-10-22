<script lang="ts">
  import IconCheckbox from './IconCheckbox.svelte';
  import InlineEdit from './InlineEdit.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import type { Item } from '$lib/menuTypes';
  import DueDatePicker from './DueDatePicker.svelte';
  import CommentPopover from './CommentPopover.svelte';
  import SubtaskList from './SubtaskList.svelte';
  import AutoDeleteDaysPopover from './AutoDeleteDaysPopover.svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { appStore } from '$stores/appStore.svelte';
  import { dragStore } from '$stores/dragStore.svelte';
  import { t } from '$i18n';
  import { fill } from '$lib/text';
  import { onEnter, onLeave } from '$lib/capture';
  import * as taskService from '$services/taskService';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import {
    canHaveSubtasks,
    formatProgress,
    getSubtaskStats,
    isClosed,
    isCompleted,
    isNotRelevant,
    isOverdue,
    isPartiallyDone,
  } from '$domain/taskRules';
  import type { Task } from '$lib/types';

  let { task, children = [] }: { task: Task; children?: Task[] } = $props();

  let editing = $state(false);
  let menuOpen = $state(false);
  let menuByKeyboard = $state(false);
  let menuAnchor = $state<DOMRect | null>(null);
  let duePickerOpen = $state(false);
  let dueAnchor = $state<DOMRect | null>(null);
  let commentOpen = $state(false);
  let commentAnchor = $state<DOMRect | null>(null);
  let daysOpen = $state(false);
  let daysAnchor = $state<DOMRect | null>(null);
  let addSubtaskOpen = $state(false);
  let newSubtaskTitle = $state('');
  let newSubtaskEl = $state<HTMLInputElement | undefined>(undefined);
  let subtaskFormEl = $state<HTMLFormElement | undefined>(undefined);

  $effect(() => {
    if (addSubtaskOpen) newSubtaskEl?.focus();
  });

  function anchorFrom(event: MouseEvent): DOMRect {
    return (event.currentTarget as HTMLElement).getBoundingClientRect();
  }

  function openDuePicker(event: MouseEvent) {
    dueAnchor = anchorFrom(event);
    duePickerOpen = true;
  }

  /** Clicking the comment icon reads it, or offers to write one. */
  function openComment(event: MouseEvent) {
    commentAnchor = anchorFrom(event);
    commentOpen = true;
  }

  const hasComment = $derived((task.comment ?? '').trim().length > 0);

  /** Right click anywhere on the row opens the task menu. */
  function openRowMenu(event: MouseEvent) {
    event.preventDefault();
    menuAnchor = anchorFrom(event);
    menuByKeyboard = false;
    menuOpen = true;
  }

  /**
   * Shift+F10 and the dedicated context-menu key open the same menu. Without
   * this the whole menu — including delete and reorder — was reachable only
   * with a mouse.
   */
  function openRowMenuByKeyboard(event: KeyboardEvent) {
    if (event.key !== 'F10' && event.key !== 'ContextMenu') return;
    if (event.shiftKey === false && event.key === 'F10') return;
    if (event.key === 'ContextMenu' && !event.shiftKey) return;
    event.preventDefault();
    const row = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuAnchor = new DOMRect(row.right, row.bottom, 0, 0);
    menuByKeyboard = true;
    menuOpen = true;
  }

  function openAddSubtask() {
    addSubtaskOpen = !addSubtaskOpen;
    if (addSubtaskOpen && !expanded) taskStore.toggleExpanded(task.id);
  }

  const stats = $derived(getSubtaskStats(children));
  const progress = $derived(formatProgress(stats, isClosed(task)));
  const partial = $derived(isPartiallyDone(task, children));
  const expanded = $derived(taskStore.isExpanded(task.id));
  const overdue = $derived(isOverdue(task));
  const notRelevant = $derived(isNotRelevant(task));
  const isSubtask = $derived(task.parentId !== null);
  const canAddSubtasks = $derived(!isSubtask && canHaveSubtasks(task));
  // Only active root tasks may adopt dragged tasks as subtasks.
  const canAdopt = $derived(canAddSubtasks);
  const isParentTarget = $derived(dragStore.hoverParentId === task.id);

  function onRowPointerDown(event: PointerEvent) {
    // The last row pressed is the keyboard's target (⌘D marks it later). Any
    // button counts: right press aims there too, since the menu it opens is
    // about this very task.
    taskStore.select(task.id);
    dragStore.begin(event, task);
  }

  function toggle() {
    void taskStore.toggle(task.id);
  }

  function toggleNotRelevant() {
    void taskStore.toggleNotRelevant(task.id);
  }

  function startEdit() {
    editing = true;
  }

  async function saveEdit(title: string) {
    await taskStore.rename(task.id, title);
    editing = false;
  }

  async function askDelete() {
    // The number shown in the confirmation must be the real one: completed
    // subtasks are not loaded when "show completed" is off, so counting the
    // rendered children would understate what is about to be deleted.
    let count = children.length;
    if (!isSubtask && count === 0) {
      try {
        count = await taskService.countSubtasks(task.id);
      } catch {
        // Fall back to what is on screen.
      }
    }
    uiStore.askDelete(task, count);
  }

  let subtaskSubmitting = $state(false);

  async function createSubtask(title: string): Promise<void> {
    // Guard against a fast typist outrunning the database: without it a held
    // Enter could create the same task twice.
    if (subtaskSubmitting) return;
    subtaskSubmitting = true;
    try {
      const ok = await taskStore.addSubtask(task.id, title);
      if (ok && !expanded) taskStore.toggleExpanded(task.id);
    } finally {
      subtaskSubmitting = false;
    }
  }

  /**
   * Enter: add the subtask and stay exactly where we are — field open, focus
   * kept, caret ready for the next one. The same one-key rhythm as the root
   * input, only inside a task.
   */
  async function submitSubtask(event?: SubmitEvent) {
    event?.preventDefault();
    const action = onEnter(newSubtaskTitle, subtaskSubmitting);
    if (action.kind !== 'add') return;
    newSubtaskTitle = '';
    await createSubtask(action.title);
    // Re-focus after the store rewrote the row: the field must never lose the
    // caret in the middle of a storm.
    queueMicrotask(() => newSubtaskEl?.focus());
  }

  /** Leaving the field finishes the gesture: save what is typed, then close. */
  async function finishSubtask() {
    const action = onLeave(newSubtaskTitle, subtaskSubmitting);
    newSubtaskTitle = '';
    if (action.kind === 'save-and-close') await createSubtask(action.title);
    addSubtaskOpen = false;
  }

  /** Escape cancels: nothing typed is created. */
  function cancelSubtask() {
    newSubtaskTitle = '';
    addSubtaskOpen = false;
  }

  /**
   * Clicking anywhere outside the inline field finishes the gesture: typed
   * text is saved, an empty field simply disappears. Without this the row kept
   * an input open until the user typed something or pressed "+" again.
   */
  $effect(() => {
    if (!addSubtaskOpen) return;
    function onPointerDown(event: MouseEvent) {
      const target = event.target as HTMLElement;
      if (subtaskFormEl && !subtaskFormEl.contains(target)) void finishSubtask();
    }
    document.addEventListener('mousedown', onPointerDown);
    return () => document.removeEventListener('mousedown', onPointerDown);
  });

  // The menu holds actions without a one-click control in the row: the
  // "не актуальная" toggle, the auto-delete setting, reorder and delete.
  // Renaming (double click), due date (calendar button), adding a subtask (+)
  // and expanding (chevron) are all reachable directly from the row.
  const menuItems: Item[] = $derived([
    {
      type: 'item' as const,
      id: 'not-relevant',
      // The mark is a toggle: the same item retires the task or puts it back
      // on the agenda, whichever direction the row asks for.
      label: notRelevant ? t('task.markRelevant') : t('task.markNotRelevant'),
      onClick: toggleNotRelevant,
    },
    { type: 'separator' as const },
    {
      type: 'item',
      id: 'auto-delete',
      label:
        settingsStore.autoDeleteDays === null
          ? t('task.autoDelete')
          : t('task.autoDeleteOn').replace('{days}', String(settingsStore.autoDeleteDays)),
      onClick: (event: MouseEvent) => {
        daysAnchor = (event.currentTarget as HTMLElement).getBoundingClientRect();
        menuOpen = false;
        daysOpen = true;
      },
    },
    ...(isSubtask
      ? []
      : [
          { type: 'separator' as const },
          {
            type: 'item' as const,
            id: 'up',
            label: t('task.moveUp'),
            onClick: () => void taskStore.move(task.id, 'up'),
          },
          {
            type: 'item' as const,
            id: 'down',
            label: t('task.moveDown'),
            onClick: () => void taskStore.move(task.id, 'down'),
          },
        ]),
    { type: 'separator' as const },
    { type: 'item', id: 'delete', label: t('task.delete'), danger: true, onClick: askDelete },
  ]);
</script>

<!-- Right click on the row opens the task menu. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="task-row"
  class:completed={isCompleted(task) && !partial && !notRelevant}
  class:overdue={overdue && !isCompleted(task) && !notRelevant}
  class:not-relevant={notRelevant}
  class:partial={partial && !notRelevant}
  class:drop-hover={isParentTarget}
  class:drop-armed={isParentTarget && dragStore.nesting}
  class:dragging={dragStore.active && dragStore.sourceId === task.id}
  class:popover-open={duePickerOpen}
  data-task-row={isSubtask ? undefined : task.id}
  data-drop-parent={canAdopt ? task.id : undefined}
  oncontextmenu={openRowMenu}
  onkeydown={openRowMenuByKeyboard}
  onpointerdown={onRowPointerDown}
>
  <div class="task-lead">
    {#if canAddSubtasks}
      <button
        class="icon-button add-subtask-trigger"
        onclick={() => openAddSubtask()}
        aria-label={t('task.addSubtask')}
        title={t('task.addSubtask')}
      >
        <svg width="12" height="12" viewBox="0 0 14 14" aria-hidden="true">
          <path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
        </svg>
      </button>
    {/if}
    <IconCheckbox
      checked={isCompleted(task)}
      partial={partial}
      notRelevant={notRelevant}
      label={fill(
        isCompleted(task) ? t('task.reopen') : t('task.complete'),
        'title',
        task.title,
      )}
      onToggle={toggle}
    />
  </div>

  <div class="task-main">
    {#if editing}
      <InlineEdit
        value={task.title}
        ariaLabel={t('task.renameLabel')}
        onSave={saveEdit}
        onCancel={() => (editing = false)}
      />
    {:else}
      <!-- Editing is also reachable from the task menu ("Редактировать"). -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span
        class="task-title"
        class:overdue={overdue}
        title={isSubtask ? task.title : `${task.title}\n${t('task.renameHint')}`}
        aria-label={fill(t('task.label'), 'title', task.title)}
        ondblclick={startEdit}
      >
        {task.title}
      </span>
    {/if}

    <!-- One chip for both states: the date when there is one, a muted "Дата"
         placeholder on hover when there is none. Click opens the picker.
         The wheel is deliberately NOT handled here: scrolling the list must
         never change a date by accident. Stepping by a day lives inside the
         popover and behind an explicit save. -->
    <button
      class="due-badge"
      class:empty={!task.dueDate}
      class:overdue={overdue}
      onclick={openDuePicker}
      title={task.dueDate
        ? overdue
          ? t('due.overdue')
          : t('due.label')
        : t('due.set')}
      aria-label={task.dueDate ? t('due.change') : t('due.set')}
    >
      <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true">
        <rect x="3" y="5" width="18" height="16" rx="2" fill="none" stroke="currentColor" stroke-width="2" />
        <path d="M3 10h18M8 3v4M16 3v4" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none" />
      </svg>
      {task.dueDate ?? t('due.placeholder')}
    </button>

    {#if progress}
      <span class="progress-badge" title={t('task.progress')}>{progress}</span>
    {/if}
  </div>

  <div class="task-actions">
    {#if children.length > 0}
      <button
        class="icon-button chevron"
        class:expanded={expanded}
        onclick={() => taskStore.toggleExpanded(task.id)}
        aria-label={expanded ? t('task.collapse') : t('task.expand')}
        aria-expanded={expanded}
      >
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <path d="m2.5 4.5 3.5 3.5 3.5-3.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    {/if}
    <button
      class="icon-button comment-trigger"
      class:filled={hasComment}
      onclick={openComment}
      aria-label={hasComment ? t('comment.open') : t('comment.add')}
      title={hasComment ? t('comment.open') : t('comment.add')}
      aria-expanded={commentOpen}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M20 4H4a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3v4l4.5-4H20a1 1 0 0 0 1-1V5a1 1 0 0 0-1-1Z"
          fill={hasComment ? 'currentColor' : 'none'}
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <button
      class="icon-button delete-trigger"
      onclick={() => void askDelete()}
      aria-label={t('task.delete')}
      title={t('task.delete')}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"
          fill="none"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>
</div>

{#if menuOpen && menuAnchor}
  <ContextMenu
    items={menuItems}
    anchor={menuAnchor}
    label={t('task.menuLabel')}
    openedByKeyboard={menuByKeyboard}
    onClose={() => (menuOpen = false)}
  />
{/if}

{#if daysOpen && daysAnchor}
  <AutoDeleteDaysPopover anchor={daysAnchor} onClose={() => (daysOpen = false)} />
{/if}

{#if duePickerOpen && dueAnchor}
  <DueDatePicker
    value={task.dueDate}
    anchor={dueAnchor}
    onPick={(date) => void taskStore.setDueDate(task.id, date)}
    onClose={() => (duePickerOpen = false)}
  />
{/if}

{#if commentOpen && commentAnchor}
  <CommentPopover
    value={task.comment}
    anchor={commentAnchor}
    onSave={(text) => void taskStore.setComment(task.id, text)}
    onClose={() => (commentOpen = false)}
  />
{/if}

{#if expanded && children.length > 0}
  <SubtaskList {children} />
{/if}

{#if addSubtaskOpen && canAddSubtasks}
  <form class="add-subtask-form" bind:this={subtaskFormEl} onsubmit={submitSubtask}>
    <input
      bind:this={newSubtaskEl}
      bind:value={newSubtaskTitle}
      placeholder={t('add.subtaskPlaceholder')}
      aria-label={t('add.subtaskLabel')}
      onkeydown={(e) => {
        // Escape must not run the form: it closes the field, nothing else.
        if (e.key === 'Escape') {
          e.preventDefault();
          cancelSubtask();
        }
      }}
      maxlength={appStore.limit.title}
    />
  </form>
  <p class="add-subtask-hint">{t('add.subtaskStormHint')}</p>
{/if}
