<script lang="ts">
  import { dragStore } from '$stores/dragStore.svelte';
  import { t } from '$i18n';

  function onMove(event: PointerEvent) {
    dragStore.move(event);
  }

  async function onUp() {
    if (dragStore.active) {
      await dragStore.drop();
    } else {
      dragStore.cancel();
    }
  }
</script>

<svelte:window
  onpointermove={onMove}
  onpointerup={() => void onUp()}
  onpointercancel={() => dragStore.cancel()}
  onblur={() => dragStore.cancel()}
  onvisibilitychange={() => dragStore.cancel()}
/>

{#if dragStore.active}
  <div
    class="drag-ghost"
    class:armed={dragStore.nesting}
    style="left: {dragStore.pointer.x + 12}px; top: {dragStore.pointer.y - 14}px;"
  >
    {#if dragStore.nesting}
      <span class="ghost-hint">{t('task.makeSubtask')}</span>
    {:else if dragStore.hoverTabId}
      <span class="ghost-hint">{t('task.moveToTab')}</span>
    {:else}
      {dragStore.title}
    {/if}
  </div>
{/if}

<style>
  .drag-ghost {
    position: fixed;
    z-index: 500;
    pointer-events: none;
    max-width: 260px;
    padding: 6px 10px;
    border-radius: 8px;
    background: var(--bg);
    border: 1px solid var(--accent);
    box-shadow: var(--shadow-menu);
    font-size: 12.5px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .drag-ghost.armed {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .ghost-hint {
    white-space: nowrap;
  }
</style>