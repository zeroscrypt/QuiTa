<script lang="ts">
  import { onMount } from 'svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import { buildDeletedHierarchy, type DeletedNode } from '$lib/trash';
  import { taskStore } from '$stores/taskStore.svelte';
  import { tabStore } from '$stores/tabStore.svelte';
  import { dbStore } from '$stores/dbStore.svelte';
  import { appStore } from '$stores/appStore.svelte';
  import type { Task } from '$lib/types';
  import { t } from '$i18n';
  import { fill } from '$lib/text';
  import { trapFocus } from '$lib/focusTrap';
  import * as taskService from '$services/taskService';
  import InfoHint from './InfoHint.svelte';

  type TabId = 'interface' | 'tasks' | 'database' | 'trash';

  const TABS: { id: TabId; label: string }[] = [
    { id: 'interface', label: t('settings.tabInterface') },
    { id: 'tasks', label: t('settings.tabTasks') },
    { id: 'database', label: t('settings.tabDatabase') },
    { id: 'trash', label: t('settings.tabTrash') },
  ];

  let active = $state<TabId>('interface');
  let purgeConfirm = $state<{ id: string; title: string } | null>(null);
  let purgeAllConfirm = $state(false);
  let purging = $state(false);
  let purgeResult = $state<string | null>(null);

  onMount(() => {
    void taskStore.loadDeleted();
    void dbStore.refresh();
    void appStore.load();
    function onKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape') uiStore.settingsOpen = false;
    }
    document.addEventListener('keydown', onKeydown);
    return () => document.removeEventListener('keydown', onKeydown);
  });

  const deleted = $derived(taskStore.deleted);
  const autoDeleteDays = $derived(settingsStore.autoDeleteDays);
  const shortcutLabel = $derived(appStore.info?.shortcut ?? '');

  /** Days left before the task is removed by the retention policy. */
  function daysLeft(task: { deletedAt: string | null }): number | null {
    if (autoDeleteDays === null) return null;
    if (!task.deletedAt) return null;
    const deletedMs = Date.parse(task.deletedAt);
    if (Number.isNaN(deletedMs)) return null;
    const elapsedDays = Math.floor((Date.now() - deletedMs) / 86_400_000);
    return Math.max(0, autoDeleteDays - elapsedDays);
  }

  function retentionLabel(task: { deletedAt: string | null }): string {
    if (autoDeleteDays === null) return t('settings.neverDeleted');
    const left = daysLeft(task);
    if (left === null) return t('settings.willDeleteOnNextRun');
    if (left <= 0) return t('settings.willDeleteOnNextRun');
    return t('settings.willDeleteIn').replace('{count}', String(left));
  }

  interface DeletedGroup {
    key: string;
    label: string;
    color: string;
    tasks: Task[];
    /** Flattened rows: a parent immediately followed by its subtasks. */
    nodes: DeletedNode[];
  }

  /** "1 подзадача" / "2 подзадачи" / "5 подзадач" — with correct plurals. */
  function childLabel(count: number): string {
    const lang = settingsStore.language;
    let key = 'settings.deletedChildrenMany';
    if (lang === 'ru') {
      const mod100 = count % 100;
      const mod10 = count % 10;
      if (mod100 < 11 || mod100 > 14) {
        if (mod10 === 1) key = 'settings.deletedChildrenOne';
        else if (mod10 >= 2 && mod10 <= 4) key = 'settings.deletedChildrenFew';
      }
    } else if (count === 1) {
      key = 'settings.deletedChildrenOne';
    }
    return t(key, { count: String(count) });
  }

  /**
   * Deleted tasks grouped by the tab they came from: the recycle bin answers
   * "what did I throw away from this project?". A tab that was later deleted
   * is reported under its former id, so nothing gets lost in an unnamed group.
   */
  const deletedGroups = $derived.by((): DeletedGroup[] => {
    const byTab = new Map<string, DeletedGroup>();
    for (const task of deleted) {
      const key = task.tabId ?? 'unknown';
      let group = byTab.get(key);
      if (!group) {
        const tab = tabStore.tabs.find((candidate) => candidate.id === task.tabId);
        group = {
          key,
          label: tab?.name ?? t('settings.unknownTab'),
          color: tab?.color ?? 'var(--text-disabled)',
          tasks: [],
          nodes: [],
        };
        byTab.set(key, group);
      }
      group.tasks.push(task);
    }

    // Rebuild the hierarchy so a subtask is shown under its deleted parent
    // instead of as an unrelated line.
    for (const group of byTab.values()) {
      group.nodes = buildDeletedHierarchy(group.tasks);
    }

    const order = new Map(tabStore.tabs.map((tab, index) => [tab.id, index]));
    return [...byTab.values()].sort(
      (a, b) =>
        (order.get(a.key) ?? Number.MAX_SAFE_INTEGER) -
        (order.get(b.key) ?? Number.MAX_SAFE_INTEGER),
    );
  });

  /** "2,4 МБ" / "812 КБ" / "340 Б" — compact and locale-independent. */
  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} ${t('db.bytes')}`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ${t('db.kilobytes')}`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} ${t('db.megabytes')}`;
  }

  /** Nothing on the free list means VACUUM has no work to do. */
  const canVacuum = $derived((dbStore.stats?.freelistCount ?? 0) > 0);

  async function purgeNow() {
    if (!purgeConfirm) return;
    purgeResult = null;
    dbStore.error = null;
    const { id } = purgeConfirm;
    purgeConfirm = null;
    try {
      await taskService.purgeTask(id);
    } catch (e) {
      dbStore.error = e instanceof Error ? e.message : String(e);
    }
    await refreshAfterPurge();
  }

  /**
   * Empties the recycle bin.
   *
   * The row count is re-read from the database afterwards instead of trusting
   * the number the command reported: a silent no-op that still says "done" is
   * worse than a visible failure, and this is the one irreversible action in
   * the app.
   */
  async function purgeAll() {
    if (purging) return;
    purging = true;
    purgeAllConfirm = false;
    purgeResult = null;
    dbStore.error = null;
    let removed = 0;
    try {
      removed = await taskService.purgeAllDeleted();
    } catch (e) {
      dbStore.error = e instanceof Error ? e.message : String(e);
    }

    try {
      await refreshAfterPurge();
    } catch (e) {
      // A failed refresh must not hide the result of the deletion itself.
      dbStore.error ??= e instanceof Error ? e.message : String(e);
    }

    // Whatever happened, the state on screen must match the database.
    const stillThere = taskStore.deleted.length;
    if (stillThere > 0) {
      dbStore.error ??= t('settings.purgeAllFailed', { count: String(stillThere) });
    } else if (!dbStore.error) {
      purgeResult = t('settings.purgeAllDone', { count: String(removed) });
    }
    purging = false;
  }

  /** One place that re-reads everything affected by a purge. */
  async function refreshAfterPurge() {
    await taskStore.loadDeleted();
    await taskStore.load();
    await dbStore.refresh();
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) uiStore.settingsOpen = false;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onBackdropClick} role="presentation">
  <div class="dialog settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" tabindex="-1" use:trapFocus>
    <div class="settings-header">
      <h2 id="settings-title" class="dialog-title">{t('settings.title')}</h2>
      <button
        class="icon-button"
        onclick={() => (uiStore.settingsOpen = false)}
        aria-label={t('settings.close')}
      >
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
        </svg>
      </button>
    </div>

    <div class="settings-tabs" role="tablist" aria-label={t('settings.title')}>
      {#each TABS as tab (tab.id)}
        <button
          class="settings-tab"
          class:active={active === tab.id}
          role="tab"
          tabindex={active === tab.id ? 0 : -1}
          aria-selected={active === tab.id}
          onclick={() => (active = tab.id)}
          onkeydown={(e) => {
            const index = TABS.findIndex((candidate) => candidate.id === active);
            if (e.key === 'ArrowRight') active = TABS[(index + 1) % TABS.length].id;
            if (e.key === 'ArrowLeft') active = TABS[(index - 1 + TABS.length) % TABS.length].id;
          }}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <div class="settings-body">
      <!-- ============================ Вид ============================ -->
      {#if active === 'interface'}
        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.language')}
            <InfoHint text={t('settings.hintLanguage')} />
          </h3>
          <div class="segmented" role="radiogroup" aria-label={t('settings.language')}>
            {#each [{ code: 'ru', label: t('settings.languageRu') }, { code: 'en', label: t('settings.languageEn') }] as option (option.code)}
              <button
                class="segment"
                class:active={settingsStore.language === option.code}
                role="radio"
                aria-checked={settingsStore.language === option.code}
                onclick={() => void settingsStore.setLanguage(option.code as 'ru' | 'en')}
              >
                {option.label}
              </button>
            {/each}
          </div>
        </section>

        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.appearance')}
            <InfoHint text={t('settings.hintTheme')} />
          </h3>
          <div class="segmented" role="radiogroup" aria-label={t('settings.appearance')}>
            {#each [{ code: 'system', label: t('settings.themeSystem') }, { code: 'light', label: t('settings.themeLight') }, { code: 'dark', label: t('settings.themeDark') }] as option (option.code)}
              <button
                class="segment"
                class:active={settingsStore.theme === option.code}
                role="radio"
                aria-checked={settingsStore.theme === option.code}
                onclick={() => void settingsStore.setTheme(option.code as 'system' | 'light' | 'dark')}
              >
                {option.label}
              </button>
            {/each}
          </div>
        </section>

        <section class="settings-section">
          <h3 class="settings-label">{t('settings.view')}</h3>
          <label class="toggle-row">
            <span class="toggle-label">
              {t('settings.showCompleted')}
              <InfoHint text={t('settings.hintShowCompleted')} />
            </span>
            <button
              class="switch"
              role="switch"
              aria-checked={settingsStore.showCompleted}
              aria-label={t('settings.showCompleted')}
              onclick={() => void settingsStore.setShowCompleted(!settingsStore.showCompleted)}
            >
              <span class="switch-thumb"></span>
            </button>
          </label>
          <label class="toggle-row">
            <span class="toggle-label">
              {t('settings.expandSubtasks')}
              <InfoHint text={t('settings.hintExpandSubtasks')} />
            </span>
            <button
              class="switch"
              role="switch"
              aria-checked={settingsStore.expandSubtasks}
              aria-label={t('settings.expandSubtasks')}
              onclick={() => void settingsStore.setExpandSubtasks(!settingsStore.expandSubtasks)}
            >
              <span class="switch-thumb"></span>
            </button>
          </label>
          <p class="settings-hint">
            {settingsStore.expandSubtasks ? t('settings.expandOn') : t('settings.expandOff')}
          </p>

          <label class="toggle-row">
            <span class="toggle-label">
              {t('settings.showTabCounts')}
              <InfoHint text={t('settings.hintShowTabCounts')} />
            </span>
            <button
              class="switch"
              role="switch"
              aria-checked={settingsStore.showTabCounts}
              aria-label={t('settings.showTabCounts')}
              onclick={() => void settingsStore.setShowTabCounts(!settingsStore.showTabCounts)}
            >
              <span class="switch-thumb"></span>
            </button>
          </label>

          <label class="toggle-row">
            <span class="toggle-label">
              {t('settings.newestFirst')}
              <InfoHint text={t('settings.hintNewestFirst')} />
            </span>
            <button
              class="switch"
              role="switch"
              aria-checked={settingsStore.newestFirst}
              aria-label={t('settings.newestFirst')}
              onclick={async () => {
                await settingsStore.setNewestFirst(!settingsStore.newestFirst);
                await taskStore.reverseRootOrder();
              }}
            >
              <span class="switch-thumb"></span>
            </button>
          </label>
          <p class="settings-hint">
            {settingsStore.newestFirst ? t('settings.newestFirstOn') : t('settings.newestFirstOff')}
          </p>
        </section>

        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.shortcut')}
            <InfoHint text={t('settings.hintShortcut')} />
          </h3>
          <p class="settings-hint">
            <!-- The combination comes from the backend, which is where it is
                 actually registered. Writing "Cmd+Shift+Q" into the markup
                 showed macOS keys on a Windows keyboard, where no such
                 combination exists. -->
            <kbd>{shortcutLabel}</kbd> — {t('settings.shortcutHint')}
          </p>
        </section>
      {/if}

      <!-- =========================== Задачи ========================== -->
      {#if active === 'tasks'}
        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.tasks')}
            <InfoHint text={t('settings.hintCascade')} />
          </h3>
          <label class="toggle-row">
            <span class="toggle-label">
              {t('settings.cascade')}
              <InfoHint text={t('settings.hintCascade')} />
            </span>
            <button
              class="switch"
              role="switch"
              aria-checked={settingsStore.cascadeComplete}
              aria-label={t('settings.cascade')}
              onclick={() => void settingsStore.setCascadeComplete(!settingsStore.cascadeComplete)}
            >
              <span class="switch-thumb"></span>
            </button>
          </label>
          <p class="settings-hint">
            {settingsStore.cascadeComplete ? t('settings.cascadeOn') : t('settings.cascadeOff')}
          </p>
        </section>

        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.autoDelete')}
            <InfoHint text={t('settings.hintAutoDelete')} />
          </h3>
          <div class="days-row">
            <input
              class="days-input"
              type="number"
              min="1"
              max="3650"
              placeholder="—"
              value={settingsStore.autoDeleteDays ?? ''}
              aria-label={t('settings.autoDeleteDays')}
              onchange={(e) => {
                const raw = (e.currentTarget as HTMLInputElement).value.trim();
                if (raw === '') {
                  void settingsStore.setAutoDeleteDays(null);
                  return;
                }
                const value = Number(raw);
                if (Number.isInteger(value) && value >= 1 && value <= 3650) {
                  void settingsStore.setAutoDeleteDays(value);
                }
              }}
            />
            <span class="days-suffix">{settingsStore.language === 'en' ? 'days' : 'дн.'}</span>
          </div>
          <p class="settings-hint">{t('settings.autoDeleteHint')}</p>
        </section>

        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.data')}
            <InfoHint text={t('settings.exportWhat')} />
          </h3>
          <div class="format-row">
            <button class="button secondary wide" onclick={() => void uiStore.runExport()}>
              {t('settings.export')}
            </button>
            <button class="button secondary wide" onclick={() => void uiStore.runImport()}>
              {t('settings.import')}
            </button>
          </div>
          <p class="settings-hint">{t('settings.dataHint')}</p>
        </section>
      {/if}

      <!-- ============================ База ============================ -->
      {#if active === 'database'}
        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.tabDatabase')}
            <InfoHint text={t('settings.hintDatabase')} />
          </h3>

          {#if dbStore.loading && !dbStore.stats}
            <p class="settings-hint">{t('common.loading')}</p>
          {:else if dbStore.stats}
            <dl class="stats-grid">
              <div class="stat">
                <dt>{t('db.size')}</dt>
                <dd>{formatBytes(dbStore.stats.sizeBytes)}</dd>
              </div>
              <div class="stat">
                <dt>{t('db.tasks')}</dt>
                <dd>{dbStore.stats.activeCount}</dd>
              </div>
              <div class="stat">
                <dt>{t('db.deleted')}</dt>
                <dd>{dbStore.stats.deletedCount}</dd>
              </div>
              <div class="stat">
                <dt>{t('db.tabs')}</dt>
                <dd>{dbStore.stats.tabCount}</dd>
              </div>
              <div class="stat">
                <dt>{t('db.free')}</dt>
                <dd>{formatBytes(dbStore.reclaimableBytes)}</dd>
              </div>
              <div class="stat">
                <dt>{t('db.pages')}</dt>
                <dd>
                  {dbStore.stats.pageCount}
                  × {formatBytes(dbStore.stats.pageSize)}
                </dd>
              </div>
            </dl>

            <button
              class="button secondary wide"
              disabled={!canVacuum || dbStore.busy}
              onclick={() => void dbStore.vacuum()}
            >
              {dbStore.busy ? t('db.vacuuming') : t('db.vacuum')}
            </button>
            <p class="settings-hint">
              {#if !canVacuum}
                {t('db.vacuumNotNeeded')}
              {:else}
                {t('db.vacuumHint').replace('{size}', formatBytes(dbStore.reclaimableBytes))}
              {/if}
            </p>
            {#if dbStore.lastVacuumSaved !== null}
              <p class="settings-hint success">
                {dbStore.lastVacuumSaved > 0
                  ? t('db.vacuumSaved').replace('{size}', formatBytes(dbStore.lastVacuumSaved))
                  : t('db.vacuumNothing')}
              </p>
            {/if}
            {#if dbStore.error}
              <p class="settings-hint error" role="alert">{dbStore.error}</p>
            {/if}
          {/if}
        </section>

        <section class="settings-section">
          <h3 class="settings-label">
            {t('db.howItWorks')}
            <InfoHint text={t('db.hintStorage')} />
          </h3>
          <p class="settings-hint">{t('db.storageNote')}</p>
          <p class="settings-hint">{t('db.vacuumScale')}</p>
        </section>
      {/if}

      <!-- =========================== Корзина ========================= -->
      {#if active === 'trash'}
        <section class="settings-section">
          <h3 class="settings-label">
            {t('settings.deleted')}
            <InfoHint text={t('settings.hintTrash')} />
          </h3>
          {#if purgeResult}
            <p class="settings-hint success">{purgeResult}</p>
          {/if}
          {#if dbStore.error}
            <p class="settings-hint error" role="alert">{dbStore.error}</p>
          {/if}
          {#if deleted.length === 0}
            <p class="settings-hint">{t('settings.deletedEmpty')}</p>
          {:else}
            <div class="deleted-header">
              <span class="settings-hint">
                {t('settings.deletedCount').replace('{count}', String(deleted.length))}
              </span>
              <button
                class="link danger"
                disabled={purging}
                onclick={() => (purgeAllConfirm = true)}
              >
                {purging ? t('settings.purging') : t('settings.purgeAll')}
              </button>
            </div>
            {#each deletedGroups as group (group.key)}
              <div class="deleted-group">
                <h4 class="deleted-group-title">
                  <span class="deleted-dot" style="background: {group.color}"></span>
                  {group.label}
                  <span class="deleted-group-count">{group.tasks.length}</span>
                </h4>
                <ul class="deleted-list" role="list">
                  {#each group.nodes as node (node.task.id)}
                    {@const task = node.task}
                    <li
                      class="deleted-row"
                      class:deleted-row-child={node.depth === 1}
                      class:deleted-row-parent={node.childCount > 0}
                    >
                      <div class="deleted-info">
                        <span class="deleted-title" title={task.title}>
                          {task.title}
                        </span>
                        <span class="deleted-meta">
                          {#if node.orphaned}
                            {t('settings.deletedSubtask')}
                          {/if}
                          {#if node.childCount > 0}
                            {childLabel(node.childCount)}
                          {/if}
                          {retentionLabel(task)}
                        </span>
                      </div>
                      <div class="deleted-actions">
                        <button
                          class="link"
                          onclick={async () => {
                            await taskStore.restore(task.id);
                            await taskStore.loadDeleted();
                            await dbStore.refresh();
                          }}
                        >
                          {t('settings.restore')}
                        </button>
                        <button
                          class="link danger"
                          onclick={() => (purgeConfirm = { id: task.id, title: task.title })}
                        >
                          {t('settings.deleteForever')}
                        </button>
                      </div>
                    </li>
                  {/each}
                </ul>
              </div>
            {/each}
            <p class="settings-hint trash-note">{t('settings.trashNote')}</p>
          {/if}
        </section>
      {/if}
    </div>
  </div>
</div>

{#if purgeAllConfirm}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => (purgeAllConfirm = false)} role="presentation">
    <div class="dialog" role="alertdialog" aria-modal="true" tabindex="-1" use:trapFocus>
      <h2 class="dialog-title">{t('delete.title')}</h2>
      <p class="dialog-message">{t('settings.purgeAllConfirm', { count: String(deleted.length) })}</p>
      <div class="dialog-actions">
        <button class="button secondary" onclick={() => (purgeAllConfirm = false)}>
          {t('delete.cancel')}
        </button>
        <button class="button danger" onclick={() => void purgeAll()}>
          {t('settings.purgeAll')}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if purgeConfirm}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => (purgeConfirm = null)} role="presentation">
    <div class="dialog" role="alertdialog" aria-modal="true" tabindex="-1" use:trapFocus>
      <h2 class="dialog-title">{t('delete.title')}</h2>
      <p class="dialog-message">
        {fill(t('settings.deleteForeverConfirm'), 'title', purgeConfirm.title)}
      </p>
      <div class="dialog-actions">
        <button class="button secondary" onclick={() => (purgeConfirm = null)}>
          {t('delete.cancel')}
        </button>
        <button class="button danger" onclick={() => void purgeNow()}>
          {t('settings.deleteForever')}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-tabs {
    display: flex;
    gap: 2px;
    padding: 0 14px;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--border);
    flex: none;
  }

  .settings-tab {
    padding: 6px 9px;
    font-size: 12.5px;
    color: var(--text-secondary);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 6px 6px 0 0;
    margin-bottom: -1px;
    cursor: default;
    transition:
      color 120ms ease,
      border-color 120ms ease;
  }

  .settings-tab:hover {
    color: var(--text);
  }

  .settings-tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
    font-weight: 550;
  }

  .settings-body {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
  }

  /* Export and import sit side by side and share the width equally. */
  .format-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    margin-bottom: 6px;
  }

  .trash-note {
    margin: 8px 0 0;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin: 0 0 12px;
  }

  .stat {
    padding: 7px 9px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 9px;
  }

  .stat dt {
    font-size: 10.5px;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    color: var(--text-disabled);
  }

  .stat dd {
    margin: 2px 0 0;
    font-size: 13px;
    font-weight: 550;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .settings-hint.success {
    color: var(--accent);
  }

  .settings-hint.error {
    color: var(--danger);
  }

  .deleted-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .deleted-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .deleted-row {
    align-items: flex-start;
  }

  /* Subtask: indented, with a connector line back to its parent. */
  .deleted-row-child {
    position: relative;
    padding-left: 18px;
    /* The connector line replaces the separator between siblings. */
    border-top: none;
  }

  .deleted-row-parent .deleted-title {
    color: var(--text-primary);
  }

  .deleted-row-child::before {
    content: '';
    position: absolute;
    left: 5px;
    top: -6px;
    bottom: 50%;
    width: 1px;
    background: var(--text-disabled);
  }

  .deleted-row-child::after {
    content: '';
    position: absolute;
    left: 5px;
    top: 50%;
    width: 8px;
    height: 1px;
    background: var(--text-disabled);
  }

  .deleted-row-child .deleted-title {
    font-size: 12.5px;
    color: var(--text-disabled);
  }

  .deleted-meta {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .deleted-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }

  .link.danger {
    color: var(--danger);
  }

  .link.danger:hover {
    background: var(--danger-soft);
  }
</style>