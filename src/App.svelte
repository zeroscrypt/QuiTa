<script lang="ts">
  import { onMount, tick } from 'svelte';
  import AppHeader from '$components/AppHeader.svelte';
  import TaskList from '$components/TaskList.svelte';
  import EmptyState from '$components/EmptyState.svelte';
  import AddTaskInput from '$components/AddTaskInput.svelte';
  import SettingsPanel from '$components/SettingsPanel.svelte';
  import ConfirmDialog from '$components/ConfirmDialog.svelte';
  import TabsBar from '$components/TabsBar.svelte';
  import AboutDialog from '$components/AboutDialog.svelte';
  import DragLayer from '$components/DragLayer.svelte';
  import UndoNotice from '$components/UndoNotice.svelte';
  import UpdateBanner from '$components/UpdateBanner.svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { tabStore } from '$stores/tabStore.svelte';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import { updateStore } from '$stores/updateStore.svelte';
  import { t } from '$i18n';
  import type { TaskNode } from '$lib/types';
  import { newestRowIndex, scrollTopForTarget } from '$lib/scroll';

  onMount(async () => {
    void uiStore.init();
    await settingsStore.init();
    await tabStore.init();
    await taskStore.init();
    // After the window is usable: a slow check must never delay the list.
    void updateStore.checkOnStart();
  });

  // Switching tabs keeps the query and re-runs it against the new tab.
  // `tabStore.activeTabId` is read explicitly: the stores below are read
  // through `untrack`, so this is the only dependency that may re-trigger it.
  $effect(() => {
    const tabId = tabStore.activeTabId;
    if (tabId && settingsStore.loaded) {
      void taskStore.reload();
      void taskStore.refreshSearch();
    }
  });

  const searching = $derived(taskStore.searchQuery.trim().length > 0);

  const searchEmpty = $derived(
    searching &&
      (taskStore.searchResult === null ||
        (taskStore.searchResult.roots.length === 0 &&
          taskStore.searchResult.matchedSubtasks.length === 0)),
  );

  // Merges root matches and subtask matches (with their parents) into one view.
  const searchView = $derived.by((): TaskNode[] => {
    const result = taskStore.searchResult;
    if (!result) return [];
    const nodes: TaskNode[] = result.roots.map((task) => ({ task, children: [] }));
    const byId = new Map<string, TaskNode>(nodes.map((n) => [n.task.id, n]));
    for (const match of result.matchedSubtasks) {
      let node = byId.get(match.parent.id);
      if (!node) {
        node = { task: match.parent, children: [] };
        byId.set(match.parent.id, node);
        nodes.push(node);
      }
      node.children.push(match.task);
    }
    return nodes.sort((a, b) => a.task.sortOrder - b.task.sortOrder);
  });

  // Reload the list when the "show completed" preference changes.
  $effect(() => {
    if (!settingsStore.loaded) return;
    settingsStore.showCompleted; // the only dependency this effect needs
    void taskStore.reload();
  });

  // Keep the reading position when rows change (completing a task must not
  // jump the list back to the top).
  let contentEl = $state<HTMLElement | undefined>(undefined);
  let savedScroll = 0;

  function onScroll(event: Event) {
    savedScroll = (event.currentTarget as HTMLElement).scrollTop;
  }

  $effect(() => {
    // Re-run whenever the visible rows change.
    void taskStore.nodes;
    void searching;
    void tick().then(() => {
      if (contentEl) contentEl.scrollTop = savedScroll;
    });
  });

  // "Jump to the newest task" from the header: scrolls the newest root row
  // into view. With "newest first" it is the first row, otherwise the last.
  $effect(() => {
    const request = uiStore.scrollToNewestRequest;
    if (request === 0) return;
    void taskStore.nodes;
    void searching;
    void tick().then(() => {
      const scroller = contentEl;
      if (!scroller) return;
      const rows = scroller.querySelectorAll<HTMLElement>('[data-task-row]');
      const index = newestRowIndex(rows.length, settingsStore.newestFirst);
      if (index === null) return;
      // Measured, not `offsetTop`: the scroller is not a positioned ancestor,
      // so offsetTop would overshoot by the container's page offset.
      scroller.scrollTo({
        top: scrollTopForTarget({
          scrollTop: scroller.scrollTop,
          scrollerTop: scroller.getBoundingClientRect().top,
          targetTop: rows[index].getBoundingClientRect().top,
        }),
        behavior: 'smooth',
      });
      // `savedScroll` follows the scroll handler while the animation runs, so
      // the restore effect does not fight it.
    });
  });
</script>

<div class="app-shell">
  <AppHeader />

  <main class="content" bind:this={contentEl} onscroll={onScroll}>
    <div id="task-panel" role="tabpanel" aria-label={t('tabs.label')}>
      {#if taskStore.loading}
        <div class="loading" role="status">{t('common.loading')}</div>
      {:else if searching}
        {#if searchEmpty}
          <EmptyState searching />
        {:else}
          <TaskList nodes={searchView} />
        {/if}
      {:else if taskStore.nodes.length === 0}
        <EmptyState />
      {:else}
        <TaskList nodes={taskStore.nodes} />
      {/if}
    </div>
  </main>

  <UndoNotice />
  <UpdateBanner />
  <TabsBar />
  <DragLayer />
  <AddTaskInput />

  <div class="toasts" aria-live="polite">
    {#if taskStore.error}
      <div class="toast error" role="alert">{taskStore.error}</div>
    {/if}
    {#if uiStore.exportMessage}
      <div class="toast" role="status">{uiStore.exportMessage}</div>
    {/if}
    {#if uiStore.importSummary}
      <div class="toast" role="status">
        {t('settings.importDone', {
          added: String(uiStore.importSummary.added),
          updated: String(uiStore.importSummary.updated),
          tabs: String(uiStore.importSummary.tabsAdded),
          tabsUpdated: String(uiStore.importSummary.tabsUpdated),
          settings: uiStore.importSummary.settingsRestored
            ? t('settings.importSettingsYes')
            : t('settings.importSettingsNo'),
        })}
      </div>
    {/if}
    {#if uiStore.exportError}
      <div class="toast error" role="alert">{uiStore.exportError}</div>
    {/if}
  </div>
</div>

{#if uiStore.settingsOpen}
  <SettingsPanel />
{/if}
{#if uiStore.aboutOpen}
  <AboutDialog onClose={() => (uiStore.aboutOpen = false)} />
{/if}
{#if uiStore.confirmDelete}
  <ConfirmDialog />
{/if}
