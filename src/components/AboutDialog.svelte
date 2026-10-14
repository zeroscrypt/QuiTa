<script lang="ts">
  import { onMount } from 'svelte';
  import { getAppInfo, type AppInfo } from '$services/appService';
  import { getLanguage, t } from '$i18n';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { updateStore } from '$stores/updateStore.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let info = $state<AppInfo | null>(null);
  const ru = $derived(getLanguage() === 'ru');

  onMount(async () => {
    try {
      info = await getAppInfo();
    } catch {
      info = null;
    }
  });

  function onBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) onClose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onBackdropClick} role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="about-title">
    <h2 id="about-title" class="dialog-title">{t('about.title')}</h2>

    <p class="about-text">
      {info ? (ru ? info.descriptionRu : info.descriptionEn) : t('app.tagline')}
    </p>

    <dl class="about-list">
      <div class="about-row">
        <dt>{t('about.version')}</dt>
        <dd>{info?.version ?? '—'}</dd>
      </div>
      <div class="about-row">
        <dt>{t('about.author')}</dt>
        <dd>{info?.author ?? '@zeroscrypt'}</dd>
      </div>
      <div class="about-row">
        <dt>{t('about.license')}</dt>
        <dd>{info?.license ?? 'MIT'}</dd>
      </div>
      <div class="about-row">
        <dt>{t('about.storage')}</dt>
        <dd>{settingsStore.language === 'en' ? 'Local SQLite file on this device' : 'Локальный файл SQLite на этом устройстве'}</dd>
      </div>
      {#if info?.repository}
        <div class="about-row">
          <dt>GitHub</dt>
          <dd class="about-link">{info.repository}</dd>
        </div>
      {/if}
    </dl>

    <div class="dialog-actions">
      <!-- The version cannot be guessed offline, so an unknown one stays
           unknown instead of claiming a number that may be wrong. -->
      <button
        class="button secondary"
        disabled={updateStore.state === 'checking'}
        onclick={() => void updateStore.check()}
      >
        {updateStore.state === 'checking' ? t('update.checking') : t('update.check')}
      </button>
      <button class="button secondary" onclick={onClose}>{t('about.close')}</button>
    </div>

    {#if updateStore.state === 'available' && updateStore.info}
      <p class="about-update">
        {t('update.available', { version: updateStore.info.version })}
      </p>
    {/if}
  </div>
</div>

<style>
  .about-text {
    margin: 0 0 14px;
    font-size: 13.5px;
    line-height: 1.6;
    color: var(--text-secondary);
  }

  .about-list {
    margin: 0 0 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .about-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12.5px;
  }

  .about-row dt {
    color: var(--text-secondary);
  }

  .about-row dd {
    margin: 0;
    font-weight: 550;
    overflow-wrap: anywhere;
    text-align: right;
  }

  .about-link {
    color: var(--accent);
  }

  .about-update {
    margin: 10px 0 0;
    padding: 7px 10px;
    border-radius: 8px;
    background: var(--accent-soft);
    border: 1px solid var(--accent);
    font-size: 12.5px;
    text-align: center;
  }
</style>