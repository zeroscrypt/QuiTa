<script lang="ts">
  import { updateStore } from '$stores/updateStore.svelte';
  import { t } from '$i18n';

  /**
   * Green strip above the tab bar, shown only when a newer release exists.
   *
   * It stays out of the way otherwise: no version check spinner, no reminder
   * about an update that was declined earlier. "Later" simply dismisses it.
   */
  const version = $derived(updateStore.info?.version ?? '');
</script>

{#if updateStore.state === 'available' || updateStore.state === 'downloading' || updateStore.state === 'error'}
  <div class="update-banner" role="status" aria-live="polite">
    {#if updateStore.state === 'downloading'}
      <span class="update-text">
        {t('update.downloading')}
        {#if updateStore.percent !== null}
          <span class="update-percent">{updateStore.percent}%</span>
        {/if}
      </span>
      <span
        class="update-progress"
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={updateStore.percent ?? undefined}
      >
        <span
          class="update-progress-fill"
          style:width={updateStore.percent !== null ? `${updateStore.percent}%` : '100%'}
        ></span>
      </span>
    {:else}
      <span class="update-text">
        {t('update.available', { version })}
      </span>
      <span class="update-actions">
        {#if updateStore.state === 'error'}
          <span class="update-error">{t('update.failed')}</span>
        {/if}
        <button class="update-button" onclick={() => void updateStore.update()}>
          {t('update.action')}
        </button>
        <button class="update-later" onclick={() => updateStore.dismiss()}>
          {t('update.later')}
        </button>
      </span>
    {/if}
  </div>
{/if}

<style>
  .update-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 12px;
    background: var(--accent-soft);
    border-bottom: 1px solid var(--accent);
    color: var(--text);
    font-size: 12.5px;
  }

  .update-text {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .update-percent {
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    margin-left: 4px;
  }

  .update-progress {
    flex: none;
    width: 90px;
    height: 4px;
    border-radius: 2px;
    background: var(--bg-secondary);
    overflow: hidden;
  }

  .update-progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 160ms ease;
  }

  .update-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .update-error {
    color: var(--danger);
  }

  .update-button {
    padding: 3px 10px;
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: var(--bg);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .update-button:hover {
    filter: brightness(1.08);
  }

  .update-later {
    padding: 3px 6px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    border-radius: 6px;
  }

  .update-later:hover {
    background: var(--bg-secondary);
    color: var(--text);
  }

  @media (prefers-reduced-motion: reduce) {
    .update-progress-fill {
      transition: none;
    }
  }
</style>