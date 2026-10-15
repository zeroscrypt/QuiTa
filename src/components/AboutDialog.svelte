<script lang="ts">
  import { onMount } from 'svelte';
  import { appStore } from '$stores/appStore.svelte';
  import { getLanguage, t } from '$i18n';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { updateStore } from '$stores/updateStore.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';

  let { onClose }: { onClose: () => void } = $props();

  const info = $derived(appStore.info);
  const ru = $derived(getLanguage() === 'ru');

  onMount(() => {
    void appStore.load();
  });

  function onBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) onClose();
  }

  /**
   * Hands the repository URL to the system browser.
   *
   * A plain `<a href>` would navigate the app's own webview to github.com,
   * replacing the window with a web page. The opener plugin is scoped to this
   * one repository, so a link to anywhere else is refused by the permission
   * layer rather than by a check in this file.
   */
  async function openRepository(url: string) {
    try {
      await openUrl(url);
    } catch {
      // The URL is shown on the same line, so a failure is not a dead end.
    }
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
          <dd>
            <button
              class="about-link"
              onclick={() => void openRepository(info!.repository)}
              title={info.repository}
            >
              {info.repository}
            </button>
          </dd>
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

    <!-- The update action lives here as well as on the green strip. The strip
         sits behind this dialog's backdrop, so from here it cannot be clicked:
         the check found a release and the only visible way to act on it has to
         be inside the dialog. -->
    {#if updateStore.state === 'available' && updateStore.info}
      <div class="about-update">
        <span>{t('update.available', { version: updateStore.info.version })}</span>
        <button class="about-update-action" onclick={() => void updateStore.update()}>
          {t('update.action')}
        </button>
      </div>
    {:else if updateStore.state === 'downloading'}
      <div class="about-update">
        <span>
          {t('update.downloading')}
          {#if updateStore.percent !== null}
            {updateStore.percent}%
          {/if}
        </span>
      </div>
    {:else if updateStore.state === 'error'}
      <div class="about-update error">
        <span>{t('update.failed')}</span>
        <button class="about-update-action" onclick={() => void updateStore.update()}>
          {t('update.retry')}
        </button>
      </div>
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
    /* A button, because a bare <a href> would replace the app window with
       github.com inside the app's own webview. */
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    font-weight: 550;
    text-align: right;
    cursor: pointer;
    overflow-wrap: anywhere;
  }

  .about-link:hover {
    text-decoration: underline;
  }

  .about-update {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin: 10px 0 0;
    padding: 7px 10px;
    border-radius: 8px;
    background: var(--accent-soft);
    border: 1px solid var(--accent);
    font-size: 12.5px;
  }

  .about-update.error {
    background: var(--danger-soft);
    border-color: var(--danger);
  }

  .about-update-action {
    flex: none;
    padding: 3px 10px;
    border: none;
    border-radius: 6px;
    background: var(--accent);
    color: var(--bg);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }

  .about-update-action:hover {
    filter: brightness(1.08);
  }
</style>