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
  import { isTabShortcut, tabForDigit } from '$lib/tabShortcut';

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

  /**
   * Escape closes the topmost layer — one layer per press, never all of them.
   * The overlays themselves used to listen on `document` and each one reacted
   * independently, so a single press could take down a date picker, the comment
   * behind it and the menu that opened them together.
   *
   * When the last layer is gone the caret goes to the task input, which is where
   * the user was heading all along. With nothing open at all, Escape falls
   * through to the input's own behaviour of clearing the field.
   */
  function onEscape(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented) return;
    if (!uiStore.dismissTopmost()) return;
    event.preventDefault();
    // A queue, not a direct call: closing the layer unmounts the component
    // that may be about to own the focus, so the next one runs after that.
    void tick().then(() => uiStore.requestFocusInput());
  }

  /**
   * ⌘1..9 on macOS, Ctrl+1..9 elsewhere — the binding every app with tabs
   * already uses, so nobody has to look it up. Both modifiers are accepted on
   * every platform: they collide with nothing here, and branching on the
   * operating system would need platform detection this app has no reason to
   * carry.
   *
   * Handled in the window rather than registered as a global shortcut on
   * purpose. A global Ctrl+3 would follow the user into their browser and
   * switch tabs in an app that is not in front.
   */
  function onWindowKeydown(event: KeyboardEvent) {
    onEscape(event);
    if (event.defaultPrevented || !isTabShortcut(event)) return;
    // A dialog owns the keyboard while it is open.
    if (
      uiStore.settingsOpen ||
      uiStore.aboutOpen ||
      uiStore.confirmDelete ||
      uiStore.confirmTabDelete
    ) {
      return;
    }

    const target = tabForDigit(tabStore.tabs, Number(event.key));
    if (!target) return;

    event.preventDefault();
    if (target !== tabStore.activeTabId) tabStore.select(target);
    // Either way the caret lands in the task input, so the next keystroke is a
    // task in the tab just selected. Re-selecting the current tab still focuses
    // it — that is the fastest way back to typing after clicking a list row.
    uiStore.requestFocusInput();
  }

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

<svelte:window onkeydown={onWindowKeydown} />

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
    {#if tabStore.error}
      <div class="toast error" role="alert">{tabStore.error}</div>
    {/if}
    {#if uiStore.exportMessage}
      <div class="toast" role="status">{uiStore.exportMessage}</div>
    {/if}
    {#if taskStore.notice}
      <div class="toast" role="status">{taskStore.notice}</div>
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
<!-- Both kinds of confirmation mount the same dialog. Mounting it on
     confirmDelete alone left a tab deletion setting its own flag with
     nothing to render it, so the menu item did nothing at all. -->
{#if uiStore.confirmDelete || uiStore.confirmTabDelete}
  <ConfirmDialog />
{/if}
