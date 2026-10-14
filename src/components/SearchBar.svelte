<script lang="ts">
  import { t } from '$i18n';
  import { taskStore } from '$stores/taskStore.svelte';

  let query = $state('');

  // Search runs on every keystroke: the database is local, so the query is
  // cheap and there is nothing to wait for.
  function onInput() {
    void taskStore.search(query);
  }

  function clear() {
    query = '';
    taskStore.clearSearch();
  }
</script>

<div class="search-bar">
  <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
    <circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" stroke-width="2" />
    <path d="m20 20-3.5-3.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
  </svg>
  <input
    class="search-input"
    bind:value={query}
    oninput={onInput}
    onkeydown={(e) => {
      if (e.key === 'Escape') clear();
    }}
    type="search"
    placeholder={t('search.placeholder')}
    aria-label={t('search.label')}
  />
  {#if query.length > 0}
    <button class="icon-button clear-button" onclick={clear} aria-label={t('search.clear')}>
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
        <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
      </svg>
    </button>
  {/if}
</div>
