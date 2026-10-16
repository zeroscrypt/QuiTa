<script lang="ts">
  import { untrack } from 'svelte';
  import type { Item } from '$lib/menuTypes';
  import { uiStore } from '$stores/uiStore.svelte';

  let {
    items,
    anchor,
    label,
    openedByKeyboard = false,
    onClose,
  }: {
    items: Item[];
    /** Viewport rect of the trigger button, used for placement. */
    anchor: DOMRect;
    /** Accessible name of the menu, e.g. "Task actions". */
    label: string;
    /** Opened with the keyboard, so the menu should take focus itself. */
    openedByKeyboard?: boolean;
    onClose: () => void;
  } = $props();

  let menuEl = $state<HTMLElement | undefined>(undefined);
  // The anchor is captured once when the menu opens; placement is refined below.
  let pos = $state(untrack(() => ({ left: anchor.right, top: anchor.bottom + 4 })));

  $effect(() => {
    function onDocumentClick(event: MouseEvent) {
      if (menuEl && !menuEl.contains(event.target as Node)) onClose();
    }
    // Registering on mount is what puts Escape handling in order: the menu sits
    // above the row and the popovers it opened, so it has to unwind first.
    const leaveLayer = uiStore.openLayer('context-menu', onClose);
    function onKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape') return;
      // Arrow keys move between items, which is what `role="menu"` promises.
      // Without this the menu was reachable only by mouse: a keyboard user
      // could not reorder a task, toggle auto-delete, or delete it at all.
      if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
      const entries = [
        ...(menuEl?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? []),
      ];
      if (entries.length === 0) return;
      event.preventDefault();
      const at = entries.indexOf(document.activeElement as HTMLElement);
      const step = event.key === 'ArrowDown' ? 1 : -1;
      const next = at === -1 ? 0 : (at + step + entries.length) % entries.length;
      entries[next].focus();
    }
    document.addEventListener('mousedown', onDocumentClick);
    document.addEventListener('keydown', onKeydown);
    return () => {
      leaveLayer();
      document.removeEventListener('mousedown', onDocumentClick);
      document.removeEventListener('keydown', onKeydown);
    };
  });

  // Land on the first item when the menu was opened by keyboard. Opening by
  // mouse leaves focus alone, so the two ways in do not fight each other.
  $effect(() => {
    if (!openedByKeyboard) return;
    queueMicrotask(() => {
      menuEl?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    });
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