<script lang="ts">
  import { untrack } from 'svelte';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import { t } from '$i18n';

  let {
    anchor,
    onClose,
  }: {
    anchor: DOMRect;
    onClose: () => void;
  } = $props();

  let popoverEl = $state<HTMLElement | undefined>(undefined);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let current = $state(untrack(() => String(settingsStore.autoDeleteDays ?? 30)));
  let pos = $state(untrack(() => ({ left: anchor.right, top: anchor.bottom + 4 })));

  const isValid = $derived(
    /^\d+$/.test(current.trim()) && Number(current.trim()) >= 1 && Number(current.trim()) <= 3650,
  );

  $effect(() => {
    inputEl?.focus();
    inputEl?.select();
    function onDocumentClick(event: MouseEvent) {
      if (popoverEl && !popoverEl.contains(event.target as Node)) onClose();
    }
    // Escape belongs to App.svelte, which unwinds one layer per press.
    const leaveLayer = uiStore.openLayer('auto-delete-days', onClose);
    document.addEventListener('mousedown', onDocumentClick);
    return () => {
      leaveLayer();
      document.removeEventListener('mousedown', onDocumentClick);
    };
  });

  $effect(() => {
    if (!popoverEl) return;
    const { width, height } = popoverEl.getBoundingClientRect();
    let left = anchor.right - width;
    let top = anchor.bottom + 4;
    if (top + height > window.innerHeight - 8) top = Math.max(8, anchor.top - height - 4);
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    pos = { left, top };
  });

  function save() {
    if (!isValid) return;
    void settingsStore.setAutoDeleteDays(Number(current.trim()));
    onClose();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="due-popover due-popover-floating"
  bind:this={popoverEl}
  style="left: {pos.left}px; top: {pos.top}px;"
  role="dialog"
  aria-label={t('settings.autoDelete')}
>
  <p class="popover-title">{t('settings.autoDelete')}</p>
  <div class="days-row">
    <input
      bind:this={inputEl}
      class="days-input"
      type="number"
      min="1"
      max="3650"
      step="1"
      bind:value={current}
      aria-label={t('settings.autoDeleteDays')}
      onkeydown={(e) => {
        if (e.key === 'Enter') save();
      }}
    />
    <span class="days-suffix">дн.</span>
  </div>
  <div class="due-actions">
    <button
      class="link"
      disabled={settingsStore.autoDeleteDays === null}
      onclick={() => {
        void settingsStore.setAutoDeleteDays(null);
        onClose();
      }}>{t('delete.cancel')}</button
    >
    <button class="link" disabled={!isValid} onclick={save}>{t('due.save')}</button>
  </div>
  {#if settingsStore.autoDeleteDays !== null}
    <p class="popover-hint">
      {t('settings.autoDeleteHint')}
    </p>
  {:else}
    <p class="popover-hint">{t('settings.neverDeleted')}</p>
  {/if}
</div>