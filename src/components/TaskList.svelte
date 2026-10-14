<script lang="ts">
  import TaskItem from './TaskItem.svelte';
  import type { TaskNode } from '$lib/types';
  import { dragStore } from '$stores/dragStore.svelte';
  import { t } from '$i18n';

  let { nodes }: { nodes: TaskNode[] } = $props();

  // The gesture itself lives in DragLayer; this component only renders the
  // drop indicator line between rows.
  function showLineBefore(index: number): boolean {
    if (!isInserting(index)) return false;
    return nodes[index]?.task.id !== dragStore.sourceId;
  }

  /**
   * Line after the very last root task. Without it "put the task at the end"
   * has no visible target, and the closest line the user can see is the one
   * above the last row — which is exactly second-to-last.
   */
  function showLineAfterLast(): boolean {
    if (!isInserting(nodes.length)) return false;
    return nodes[nodes.length - 1]?.task.id !== dragStore.sourceId;
  }

  function isInserting(index: number): boolean {
    if (!dragStore.active || dragStore.zone !== 'insert') return false;
    if (dragStore.hoverTabId) return false;
    // Only the root list: a subtask group has its own indicator.
    if (dragStore.dropParentId !== null) return false;
    return dragStore.dropIndex === index;
  }
</script>

<ul class="task-list" role="list" aria-label={t('add.label')}>
  {#each nodes as node, index (node.task.id)}
    <li>
      {#if showLineBefore(index)}
        <div class="drop-line"></div>
      {/if}
      <TaskItem task={node.task} children={node.children} />
    </li>
  {/each}
  {#if showLineAfterLast()}
    <div class="drop-line"></div>
  {/if}
</ul>

<style>
  .task-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  /* Extra breathing room between root tasks: the top/bottom quarters of a row
     are the "insert" zones, so they need to be comfortable targets. */
  .task-list > li {
    padding: 3px 0;
  }

  .drop-line {
    height: 2px;
    margin: 1px 8px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>