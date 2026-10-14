<script lang="ts">
  import { uiStore } from '$stores/uiStore.svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { truncate } from '$domain/taskRules';
  import { t } from '$i18n';
  import { fill } from '$lib/text';

  const deleteState = $derived(uiStore.confirmDelete);
  let busy = $state(false);
  let confirmBtn = $state<HTMLButtonElement | undefined>(undefined);

  // Enter confirms the destructive action without touching the mouse.
  function onDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !busy) {
      event.preventDefault();
      void confirm();
    }
  }

  $effect(() => {
    if (deleteState) confirmBtn?.focus();
  });

  async function confirm() {
    if (!deleteState || busy) return;
    busy = true;
    try {
      await taskStore.remove(deleteState.task.id);
    } finally {
      busy = false;
      uiStore.cancelDelete();
    }
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) uiStore.cancelDelete();
  }
</script>

{#if deleteState}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onBackdropClick} role="presentation">
    <div
    class="dialog"
    tabindex="-1"
    onkeydown={onDialogKeydown}
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
  >
      <h2 id="confirm-title" class="dialog-title">{t('delete.title')}</h2>
      <p class="dialog-message">
        {fill(
          fill(t('delete.message'), 'title', truncate(deleteState.task.title, 60)),
          'cascade',
          deleteState.cascadeCount > 0
            ? fill(t('delete.cascade'), 'count', String(deleteState.cascadeCount))
            : '',
        )}
      </p>
      <div class="dialog-actions">
        <button class="button secondary" onclick={() => uiStore.cancelDelete()}>{t('delete.cancel')}</button>
        <button class="button danger" bind:this={confirmBtn} disabled={busy} onclick={() => void confirm()}>
          {busy ? t('delete.working') : t('delete.confirm')}
        </button>
      </div>
    </div>
  </div>
{/if}
