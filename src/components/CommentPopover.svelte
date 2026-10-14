<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '$i18n';

  let {
    value,
    anchor,
    onSave,
    onClose,
  }: {
    value: string | null;
    anchor: DOMRect;
    /** Called with the text to store, or `null` to remove the comment. */
    onSave: (text: string | null) => void;
    onClose: () => void;
  } = $props();

  let popoverEl = $state<HTMLElement | undefined>(undefined);
  let areaEl = $state<HTMLTextAreaElement | undefined>(undefined);
  /** An existing comment opens for reading; an empty one starts in edit mode. */
  let editing = $state(untrack(() => (value ?? '').trim().length === 0));
  let draft = $state(untrack(() => value ?? ''));
  let savedText = $state(untrack(() => value ?? ''));
  let pos = $state(untrack(() => ({ left: anchor.right, top: anchor.bottom + 4 })));
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  /** Writes the draft if it differs from what is stored. */
  function flush(): void {
    const text = draft.trim();
    const next = text.length === 0 ? null : text;
    if ((savedText.trim() || null) === next) return;
    savedText = text;
    onSave(next);
  }

  /** Debounced autosave: typing is saved without a button. */
  function scheduleSave(): void {
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      flush();
    }, 400);
  }

  function close(): void {
    if (saveTimer !== null) clearTimeout(saveTimer);
    saveTimer = null;
    flush();
    onClose();
  }

  $effect(() => {
    if (editing) areaEl?.focus();
    // Any click outside dismisses the popover — and keeps what was typed.
    function onPointerDown(event: MouseEvent) {
      if (popoverEl && !popoverEl.contains(event.target as Node)) close();
    }
    function onKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape') close();
    }
    document.addEventListener('mousedown', onPointerDown);
    document.addEventListener('keydown', onKeydown);
    return () => {
      if (saveTimer !== null) clearTimeout(saveTimer);
      document.removeEventListener('mousedown', onPointerDown);
      document.removeEventListener('keydown', onKeydown);
    };
  });

  /**
   * Grows to the left of the icon, the same way the date picker does, so the
   * trash button next to it stays clickable.
   */
  $effect(() => {
    if (!popoverEl) return;
    const { width, height } = popoverEl.getBoundingClientRect();
    const fitsLeft = anchor.right - width >= 8;
    let left = fitsLeft ? anchor.right - width : anchor.left;
    let top = anchor.bottom + 6;
    if (top + height > window.innerHeight - 8) {
      top = Math.max(8, anchor.top - height - 6);
    }
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    pos = { left, top };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="comment-popover comment-popover-floating"
  bind:this={popoverEl}
  style="left: {pos.left}px; top: {pos.top}px;"
  role="dialog"
  aria-label={t('comment.label')}
>
  {#if editing}
    <p class="popover-title">{t('comment.label')}</p>
    <textarea
      bind:this={areaEl}
      class="comment-area"
      bind:value={draft}
      oninput={scheduleSave}
      placeholder={t('comment.placeholder')}
      aria-label={t('comment.label')}
      rows="4"
      maxlength="2000"
    ></textarea>
    <p class="comment-hint">{t('comment.autosave')}</p>
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="comment-body"
      ondblclick={() => (editing = true)}
      title={t('comment.editHint')}
    >
      {value}
    </div>
  {/if}
</div>

<style>
  .comment-popover {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 236px;
    cursor: default;
  }

  .popover-title {
    margin: 0;
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .comment-body {
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 200px;
    overflow-y: auto;
    border-radius: 6px;
  }

  .comment-body:hover {
    background: var(--accent-soft);
  }

  .comment-area {
    width: 100%;
    padding: 7px 9px;
    resize: vertical;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 7px;
    outline: none;
    font: inherit;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text);
  }

  .comment-area:focus-visible {
    border-color: var(--accent);
  }

  .comment-hint {
    margin: 0;
    font-size: 11px;
    color: var(--text-disabled);
  }
</style>