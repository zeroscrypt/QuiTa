<script lang="ts">
  import { untrack } from 'svelte';
  import type { Item } from '$lib/menuTypes';

  let {
    items,
    anchor,
    label,
    onClose,
  }: {
    items: Item[];
    /** Viewport rect of the trigger button, used for placement. */
    anchor: DOMRect;
    /** Accessible name of the menu, e.g. "Task actions". */
    label: string;
    onClose: () => void;
  } = $props();

  let menuEl = $state<HTMLElement | undefined>(undefined);
  // The anchor is captured once when the menu opens; placement is refined below.
  let pos = $state(untrack(() => ({ left: anchor.right, top: anchor.bottom + 4 })));

  $effect(() => {
    function onDocumentClick(event: MouseEvent) {
      if (menuEl && !menuEl.contains(event.target as Node)) onClose();
    }
    function onKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose();
    }
    document.addEventListener('mousedown', onDocumentClick);
    document.addEventListener('keydown', onKeydown);
    return () => {
      document.removeEventListener('mousedown', onDocumentClick);
      document.removeEventListener('keydown', onKeydown);
    };
  });

  // Align to the trigger's right edge and flip upwards when there is no room.
  $effect(() => {
    if (!menuEl) return;
    const { width, height } = menuEl.getBoundingClientRect();
    let left = anchor.right - width;
    let top = anchor.bottom + 4;
    if (top + height > window.innerHeight - 8) {
      top = Math.max(8, anchor.top - height - 4);
    }
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    pos = { left, top };
  });
</script>

<div
  class="menu menu-floating"
  bind:this={menuEl}
  style="left: {pos.left}px; top: {pos.top}px;"
  role="menu"
  aria-label={label}
>
  {#each items as item, i (item.type === 'separator' ? `sep-${i}` : item.id)}
    {#if item.type === 'separator'}
      <div class="menu-separator"></div>
    {:else}
      <button
        class="menu-item"
        class:danger={item.danger}
        role="menuitem"
        disabled={item.disabled}
        onclick={(event) => {
          item.onClick(event);
          onClose();
        }}
      >
        {item.label}
      </button>
    {/if}
  {/each}
</div>