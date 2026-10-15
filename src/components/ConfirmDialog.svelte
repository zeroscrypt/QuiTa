<script lang="ts">
  import { uiStore } from '$stores/uiStore.svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { tabStore } from '$stores/tabStore.svelte';
  import { truncate } from '$domain/taskRules';
  import { t } from '$i18n';
  import { fill } from '$lib/text';

  const deleteState = $derived(uiStore.confirmDelete);
  const tabState = $derived(uiStore.confirmTabDelete);
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
    if (deleteState || tabState) confirmBtn?.focus();
  });

  async function confirm() {
    if (busy) return;
    const task = deleteState;
    const tab = tabState;
    if (!task && !tab) return;
    busy = true;
    try {
      if (tab) {
        const deletion = await tabStore.remove(tab.tab.id);
        // The notice holds everything needed to undo, so the dialog can close
        // before the countdown starts.
        if (deletion) uiStore.announceTabDeletion(deletion);
      } else if (task) {
        await taskStore.remove(task.task.id);
      }
    } finally {
      busy = false;
      uiStore.cancelDelete();
      uiStore.cancelTabDelete();
    }
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target !== event.currentTarget) return;
    uiStore.cancelDelete();
    uiStore.cancelTabDelete();
  }
</script>

{#if deleteState || tabState}
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
      <h2 id="confirm-title" class="dialog-title">
        {tabState ? t('delete.tabTitle') : t('delete.title')}
      </h2>
      <p class="dialog-message">
        {#if tabState}
          {fill(
            fill(t('delete.tabMessage'), 'title', truncate(tabState.tab.name, 60)),
            'tasks',
            tabState.taskCount > 0
              ? fill(t('delete.tabTasks'), 'count', String(tabState.taskCount))
              : '',
          )}
        {:else if deleteState}
          {fill(
            fill(t('delete.message'), 'title', truncate(deleteState.task.title, 60)),
            'cascade',
            deleteState.cascadeCount > 0
              ? fill(t('delete.cascade'), 'count', String(deleteState.cascadeCount))
              : '',
          )}
        {/if}
      </p>
      <div class="dialog-actions">
        <button class="button secondary" onclick={() => uiStore.cancelDelete()}>{t('delete.cancel')}</button>
        <button class="button danger" bind:this={confirmBtn} disabled={busy} onclick={() => void confirm()}>
          {busy
            ? t('delete.working')
            : tabState
              ? t('delete.tabConfirm')
              : t('delete.confirm')}
        </button>
      </div>
    </div>
  </div>
{/if}
