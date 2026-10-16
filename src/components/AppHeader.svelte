<script lang="ts">
  import SearchBar from './SearchBar.svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { toggleWindowHeight } from '$services/windowService';
  import { t } from '$i18n';

  /** True when at least one task with subtasks is closed. */
  const anyCollapsed = $derived.by(() => {
    const withChildren = taskStore.nodes.filter((node) => node.children.length > 0);
    if (withChildren.length === 0) return false;
    return withChildren.some((node) => !taskStore.isExpanded(node.task.id));
  });

  function toggleAll() {
    if (anyCollapsed) taskStore.expandAll();
    else taskStore.collapseAll();
  }

  /**
   * Jumps back to the newest task. With "newest on top" this is the first row,
   * otherwise it is the last one.
   */
  function scrollToNewest() {
    uiStore.requestScrollToNewest();
  }

  function onHeaderDoubleClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (!target?.closest('[data-drop-logo]')) return;
    void toggleWindowHeight();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  class="app-header"
  data-tauri-drag-region
  ondblclick={onHeaderDoubleClick}
>
  <!-- The logo is the only resize target, but the listener has to sit on the
       drag region itself: a handler on this span is swallowed by the window
       drag and the double click turns into a plain move. So the header
       receives the event and ignores it unless it started here. -->
  <span class="app-title" data-drop-logo title={t('window.resizeHint')}>
    {t('app.name')}
  </span>

  <SearchBar />

  <!-- One button for both directions: the arrow shows what the click will do. -->
  <button
    class="icon-button header-button"
    onclick={toggleAll}
    aria-label={anyCollapsed ? t('task.expandAll') : t('task.collapseAll')}
    title={anyCollapsed ? t('task.expandAll') : t('task.collapseAll')}
  >
    {#if anyCollapsed}
      <svg width="15" height="15" viewBox="0 0 14 14" aria-hidden="true">
        <path
          d="M2 5.5 7 1l5 4.5M2 9.5 7 14l5-4.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {:else}
      <svg width="15" height="15" viewBox="0 0 14 14" aria-hidden="true">
        <path
          d="M2 8.5 7 13l5-4.5M2 5.5 7 1l5 4.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {/if}
  </button>

  <button
    class="icon-button header-button"
    onclick={scrollToNewest}
    aria-label={t('task.toNewest')}
    title={t('task.toNewest')}
  >
    <!-- Arrow pointing to the edge where new tasks land. -->
    {#if settingsStore.newestFirst}
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 5v14M12 5l-6 6M12 5l6 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    {:else}
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 19V5M12 19l-6-6M12 19l6-6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    {/if}
  </button>

  <button
    class="icon-button settings-button"
    onclick={() => uiStore.openSettings()}
    aria-label={t('settings.title')}
    title={t('settings.title')}
  >
    <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
      <path
        fill="currentColor"
        d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.5.5 0 0 0 .12-.64l-1.92-3.32a.5.5 0 0 0-.61-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.5.5 0 0 0-.5-.42h-3.84a.5.5 0 0 0-.5.42l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96a.5.5 0 0 0-.61.22L2.65 8.84a.5.5 0 0 0 .12.64l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.5.5 0 0 0-.12.64l1.92 3.32c.13.23.4.32.61.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.25.42.5.42h3.84c.25 0 .45-.18.5-.42l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.1.48.01.61-.22l1.92-3.32a.5.5 0 0 0-.12-.64l-2.03-1.58ZM12 15.6A3.6 3.6 0 1 1 12 8.4a3.6 3.6 0 0 1 0 7.2Z"
      />
    </svg>
  </button>
</header>