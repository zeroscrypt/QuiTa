<script lang="ts">
  import { t } from '$i18n';
  let { searching = false }: { searching?: boolean } = $props();
</script>

<div class="empty-state">
  {#if searching}
    <p class="empty-title">{t('empty.search.title')}</p>
    <p class="empty-hint">{t('empty.search.hint')}</p>
  {:else}
    <p class="empty-title">{t('empty.clean.title')}</p>
    <p class="empty-hint">
      {t('empty.clean.hint')}
      <br />
      <kbd>Cmd</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd>
    </p>
  {/if}
</div>
