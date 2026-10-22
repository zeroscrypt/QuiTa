<script lang="ts">
  import { onMount } from 'svelte';
  import { appStore } from '$stores/appStore.svelte';
  import { t } from '$i18n';
  import { updateStore } from '$stores/updateStore.svelte';
  import { trapFocus } from '$lib/focusTrap';
  import AboutBody from './AboutBody.svelte';

  let { onClose }: { onClose: () => void } = $props();

  onMount(() => {
    void appStore.load();
  });

  function onBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) onClose();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onBackdropClick} role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="about-title" tabindex="-1" use:trapFocus>
    <h2 id="about-title" class="dialog-title">{t('about.title')}</h2>

    <AboutBody />

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
  </div>
</div>
