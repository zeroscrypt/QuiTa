<script lang="ts">
  import TaskItem from './TaskItem.svelte';
  import { dragStore } from '$stores/dragStore.svelte';
  import type { Task } from '$lib/types';
  import { t } from '$i18n';

  let { children }: { children: Task[] } = $props();

  const parentId = $derived(children[0]?.parentId ?? null);

  /** Insertion line before this child while a sibling is being dragged. */
  function showLineBefore(index: number): boolean {
    if (!isInserting(index)) return false;
    return children[index]?.id !== dragStore.sourceId;
  }

  /** Same reason as in the root list: "append to this group" must be visible. */
  function showLineAfterLast(): boolean {
    if (!isInserting(children.length)) return false;
    return children[children.length - 1]?.id !== dragStore.sourceId;
  }

  function isInserting(index: number): boolean {
    if (!dragStore.active || dragStore.zone !== 'insert') return false;
    if (dragStore.dropParentId !== parentId) return false;
    return dragStore.dropIndex === index;
  }
</script>

<div class="subtasks" role="group" aria-label={t('task.subtasksLabel')}>
  {#each children as child, index (child.id)}
    {#if showLineBefore(index)}
      <div class="drop-line subtask-drop-line" aria-hidden="true"></div>
    {/if}
    <!-- Marked so the drag layer can reorder siblings: the whole group
         belongs to `data-subtask-parent`. -->
    <div
      class="subtask-row"
      data-subtask-row={child.id}
      data-subtask-parent={parentId ?? ''}
    >
      <TaskItem task={child} children={[]} />
    </div>
  {/each}
  {#if showLineAfterLast()}
    <div class="drop-line subtask-drop-line" aria-hidden="true"></div>
  {/if}
</div>