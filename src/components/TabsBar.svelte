<script lang="ts">
  import { tick, untrack } from 'svelte';
  import ContextMenu from './ContextMenu.svelte';
  import { tabStore } from '$stores/tabStore.svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import { appStore } from '$stores/appStore.svelte';
  import { dragStore } from '$stores/dragStore.svelte';
  import { settingsStore } from '$stores/settingsStore.svelte';
  import type { Item } from '$lib/menuTypes';
  import { normalizeTitle } from '$domain/taskValidation';
  import { t } from '$i18n';

  let editing = $state<string | null>(null);
  let draft = $state('');
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let menuAnchor = $state<DOMRect | null>(null);
  let menuByKeyboard = $state(false);
  let menuTabId = $state<string | null>(null);
  let stripEl = $state<HTMLElement | undefined>(undefined);
  /** Edge fades, shown only when there is more to scroll towards. */
  let overflow = $state({ left: false, right: false });

  const tabs = $derived(tabStore.tabs);
  const canDelete = $derived(tabs.length > 1);

  function measureOverflow() {
    const el = stripEl;
    if (!el) {
      overflow = { left: false, right: false };
      return;
    }
    const max = el.scrollWidth - el.clientWidth;
    overflow = {
      left: el.scrollLeft > 1,
      right: max > 1 && el.scrollLeft < max - 1,
    };
  }

  /** Clicking an edge arrow scrolls roughly one screen towards that side. */
  function scrollStrip(direction: -1 | 1) {
    const el = stripEl;
    if (!el) return;
    const step = Math.max(120, Math.round(el.clientWidth * 0.7));
    el.scrollBy({ left: step * direction, behavior: 'smooth' });
    // The scroll event also calls measureOverflow once the animation lands.
    queueMicrotask(measureOverflow);
  }

  /**
   * The wheel scrolls the strip sideways. A vertical wheel gesture would
   * otherwise scroll the task list underneath, so the event is consumed —
   * which needs a non-passive listener.
   */
  function onStripWheel(event: WheelEvent) {
    const el = stripEl;
    if (!el) return;
    const max = el.scrollWidth - el.clientWidth;
    if (max <= 0) return;
    // Trackpads already report horizontal deltas; a mouse reports vertical.
    const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
    if (delta === 0) return;
    event.preventDefault();
    const before = el.scrollLeft;
    el.scrollLeft = before + delta;
    if (el.scrollLeft !== before) measureOverflow();
  }

  // Keeps the selected tab visible: after adding, renaming or switching.
  $effect(() => {
    const activeId = tabStore.activeTabId;
    const count = tabs.length;
    void count;
    if (!activeId) return;
    void tick().then(() => {
      const el = stripEl;
      const tabEl = el?.querySelector<HTMLElement>(`[data-tab-item="${CSS.escape(activeId)}"]`);
      if (!el || !tabEl) return;
      const left = tabEl.offsetLeft;
      const right = left + tabEl.offsetWidth;
      if (left < el.scrollLeft) el.scrollLeft = left - 8;
      else if (right > el.scrollLeft + el.clientWidth) el.scrollLeft = right - el.clientWidth + 8;
      measureOverflow();
    });
  });

  $effect(() => {
    const el = stripEl;
    if (!el) return;
    measureOverflow();
    const observer = new ResizeObserver(measureOverflow);
    observer.observe(el);
    // `passive: false` is required to call preventDefault on a wheel event;
    // Svelte's own attribute binding would mark it passive.
    el.addEventListener('wheel', onStripWheel, { passive: false });
    return () => {
      observer.disconnect();
      el.removeEventListener('wheel', onStripWheel);
    };
  });

  async function select(tabId: string) {
    tabStore.select(tabId);
  }

  function startRename(tabId: string, name: string) {
    editing = tabId;
    draft = untrack(() => name);
    queueMicrotask(() => inputEl?.focus());
  }

  function startCreate() {
    editing = 'new';
    draft = '';
    queueMicrotask(() => inputEl?.focus());
  }

  async function commit() {
    const name = normalizeTitle(draft ?? '');
    const target = editing;
    editing = null;
    if (!target || !name) return;
    if (target === 'new') {
      await tabStore.create(name);
    } else {
      await tabStore.rename(target, name);
    }
  }

  function openMenu(event: MouseEvent, tabId: string) {
    event.preventDefault();
    menuTabId = tabId;
    menuAnchor = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuByKeyboard = false;
  }

  // --- Drag & drop to reorder tabs -------------------------------------------
  const THRESHOLD = 5;
  let dragId = $state<string | null>(null);
  let dropIndex = $state<number | null>(null);
  let startX = 0;
  let armed = false;

  /** Insertion line before the tab at `index`, while reordering tabs. */
  function showLineBefore(index: number): boolean {
    if (!dragId || dropIndex !== index) return false;
    return tabs[index]?.id !== dragId;
  }

  /** done/total counter of a tab. */
  function statFor(id: string): { done: number; total: number } | null {
    return tabStore.statFor(id);
  }

  /** The line can also land after the last tab. */
  function showLineAfterLast(): boolean {
    if (!dragId || dropIndex !== tabs.length) return false;
    return tabs[tabs.length - 1]?.id !== dragId;
  }

  function tabUnder(x: number, y: number): HTMLElement | null {
    const element = document.elementFromPoint(x, y) as HTMLElement | null;
    return element?.closest<HTMLElement>('[data-tab-item]') ?? null;
  }

  /**
   * WAI-ARIA tab pattern: Left/Right move between tabs, Home/End jump to the
   * ends. Only the selected tab is in the tab order (roving tabindex), so Tab
   * leaves the strip in a single step.
   */
  /**
 * Renaming or deleting a tab was reachable only by mouse: the menu opened on
 * right click, and on double click alone. Shift+F10 and the dedicated
 * context-menu key open the same menu for anyone not using a mouse.
 */
function openMenuByKeyboard(event: KeyboardEvent, tabId: string): boolean {
    const wanted = event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey);
    if (!wanted) return false;
    event.preventDefault();
    menuTabId = tabId;
    menuAnchor = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuByKeyboard = true;
    return true;
  }

  function onTabKeydown(event: KeyboardEvent, index: number) {
    const tab = tabs[index];
    if (tab && openMenuByKeyboard(event, tab.id)) return;
    const last = tabs.length - 1;
    let next = index;
    switch (event.key) {
      case 'ArrowRight':
        next = Math.min(index + 1, last);
        break;
      case 'ArrowLeft':
        next = Math.max(index - 1, 0);
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = last;
        break;
      case 'Enter':
      case ' ':
        next = index;
        break;
      default:
        return;
    }
    event.preventDefault();
    const target = tabs[next];
    if (!target) return;
    select(target.id);
    void tick().then(() => {
      document.querySelector<HTMLElement>(`[data-tab-item="${CSS.escape(target.id)}"]`)?.focus();
    });
  }

  function onTabPointerDown(event: PointerEvent, tabId: string) {
    if (event.button !== 0) return;
    if ((event.target as HTMLElement).closest('input')) return;
    armed = true;
    startX = event.clientX;
    dragId = null;
    void tabId;
  }

  function onTabPointerMove(event: PointerEvent) {
    if (!armed) return;
    // A button released outside the window never produces `pointerup` here.
    // Without this guard `armed` would stay true and the next ordinary mouse
    // move would start a phantom drag, ending in an unsaved-looking reorder.
    if (event.buttons === 0) {
      resetDrag();
      return;
    }
    if (!dragId) {
      if (Math.abs(event.clientX - startX) < THRESHOLD) return;
      dragId = tabUnder(event.clientX, event.clientY)?.dataset.tabItem ?? null;
      if (!dragId) {
        resetDrag();
        return;
      }
    }
    const el = tabUnder(event.clientX, event.clientY);
    dropIndex = el ? insertIndex(el, event.clientX) : null;
    event.preventDefault();
  }

  /** Aborts a tab drag without touching the stored order. */
  function resetDrag() {
    armed = false;
    dragId = null;
    dropIndex = null;
  }

  function insertIndex(el: HTMLElement, x: number): number {
    const items = Array.from(document.querySelectorAll<HTMLElement>('[data-tab-item]'));
    const index = items.indexOf(el);
    if (index < 0) return items.length;
    const rect = items[index].getBoundingClientRect();
    return x > rect.left + rect.width / 2 ? index + 1 : index;
  }

  async function onTabPointerUp() {
    const source = dragId;
    const target = dropIndex;
    dragId = null;
    dropIndex = null;
    armed = false;
    if (!source || target === null) return;

    const ids = tabs.map((tab) => tab.id);
    const from = ids.indexOf(source);
    let to = target;
    if (from < 0) return;
    if (to > from) to -= 1;
    if (to === from) return;
    ids.splice(to, 0, ...ids.splice(from, 1));
    await tabStore.reorder(ids);
  }

  const menuItems: Item[] = $derived(
    menuTabId
      ? [
          {
            type: 'item',
            id: 'rename',
            label: t('tab.rename'),
            onClick: () => {
              const tab = tabs.find((t) => t.id === menuTabId);
              if (tab) startRename(tab.id, tab.name);
            },
          },
          {
            type: 'item',
            id: 'delete',
            label: canDelete ? t('tab.delete') : t('tab.deleteLast'),
            danger: canDelete,
            disabled: !canDelete,
            onClick: async () => {
              if (!menuTabId || !canDelete) return;
              const tab = tabs.find((x) => x.id === menuTabId);
              if (!tab) return;
              // The count is fetched rather than read from a badge that may be
              // stale, so the warning cannot promise fewer tasks than go.
              await tabStore.refreshStats();
              uiStore.askDeleteTab(tab, tabStore.stats[tab.id]?.total ?? 0);
              },
            },
          ]
        : [],
    );
  </script>

<svelte:window
  onpointermove={onTabPointerMove}
  onpointerup={() => void onTabPointerUp()}
  onpointercancel={resetDrag}
  onblur={resetDrag}
/>

<div class="tabs-bar">
  {#if overflow.left}
    <button
      class="tab-scroll left"
      onclick={() => scrollStrip(-1)}
      aria-label={t('tab.scrollLeft')}
      title={t('tab.scrollLeft')}
    >
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M15 5l-7 7 7 7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  {/if}
  <div
    class="tabs-strip"
    bind:this={stripEl}
    role="tablist"
    aria-label={t('tabs.label')}
    onscroll={measureOverflow}
  >
    {#each tabs as tab, index (tab.id)}
      {#if showLineBefore(index)}
        <div class="tab-drop-line" aria-hidden="true"></div>
      {/if}
      {#if editing === tab.id}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="tab editing">
          <span class="tab-dot" style="background: {tab.color}"></span>
          <input
            bind:this={inputEl}
            class="tab-input"
            bind:value={draft}
            aria-label={t('tab.nameLabel')}
            maxlength={appStore.limit.tabName}
            onkeydown={(e) => {
              if (e.key === 'Enter') void commit();
              if (e.key === 'Escape') editing = null;
            }}
            onblur={() => void commit()}
          />
        </div>
      {:else}
        <div
          class="tab"
          class:active={tab.id === tabStore.activeTabId}
          class:drop-target={dragStore.hoverTabId === tab.id}
          class:dragging={dragId === tab.id}
          role="tab"
          tabindex={tab.id === tabStore.activeTabId ? 0 : -1}
          aria-selected={tab.id === tabStore.activeTabId}
          aria-controls="task-panel"
          data-drop-tab={tab.id}
          data-tab-item={tab.id}
          title={`${tab.name} — ${t('task.renameHint')}`}
          ondblclick={() => startRename(tab.id, tab.name)}
          oncontextmenu={(e) => openMenu(e, tab.id)}
          onclick={() => {
            select(tab.id);
            // Clicking a tab with the mouse should leave the keyboard ready,
            // the same way the number shortcut does. Without this the user has
            // to click the input again before typing, which is the second click
            // that selecting a tab with the keyboard never needs.
            uiStore.requestFocusInput();
          }}
          onpointerdown={(e) => onTabPointerDown(e, tab.id)}
          onkeydown={(e) => onTabKeydown(e, index)}
        >
          <span class="tab-dot" style="background: {tab.color}"></span>
          <span class="tab-name">{tab.name}</span>
          {#if settingsStore.showTabCounts && statFor(tab.id)}
            <!-- done/total; subtasks count as tasks, deleted ones do not -->
            <span class="tab-count" title={t('tab.countHint')}>
              {statFor(tab.id)?.done}/{statFor(tab.id)?.total}
            </span>
          {/if}
        </div>
      {/if}
    {/each}

    {#if showLineAfterLast()}
      <div class="tab-drop-line" aria-hidden="true"></div>
    {/if}

    {#if editing === 'new'}
      <div class="tab editing">
        <input
          bind:this={inputEl}
          class="tab-input"
          bind:value={draft}
          placeholder={t('tab.new')}
          aria-label={t('tab.nameLabel')}
          maxlength={appStore.limit.tabName}
          onkeydown={(e) => {
            if (e.key === 'Enter') void commit();
            if (e.key === 'Escape') editing = null;
          }}
          onblur={() => void commit()}
        />
      </div>
    {/if}
  </div>

  {#if overflow.right}
    <button
      class="tab-scroll right"
      onclick={() => scrollStrip(1)}
      aria-label={t('tab.scrollRight')}
      title={t('tab.scrollRight')}
    >
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M9 5l7 7-7 7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  {/if}

  <button
    class="tab-add"
    onclick={startCreate}
    aria-label={t('tab.add')}
    title={t('tab.add')}
  >
    <svg width="13" height="13" viewBox="0 0 14 14" aria-hidden="true">
      <path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
    </svg>
  </button>
</div>

{#if menuAnchor && menuTabId}
  <ContextMenu
    items={menuItems}
    anchor={menuAnchor}
    openedByKeyboard={menuByKeyboard}
    label={t('tab.menuLabel')}
    onClose={() => (menuAnchor = null)}
  />
{/if}

<style>
  .tabs-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 10px 0;
    border-top: 1px solid var(--border);
    background: var(--bg);
    flex: none;
  }

  /* The strip scrolls horizontally when there are many tabs, the add button
     stays pinned on the right. */
  .tabs-strip {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1 1 auto;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    padding-bottom: 4px;
  }

  .tabs-strip::-webkit-scrollbar {
    display: none;
  }

  /* Arrows show that tabs continue beyond the window. Each one appears only
     when there is actually something to scroll towards. */
  .tab-scroll {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 22px;
    padding: 0;
    flex: none;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: default;
    transition:
      color 120ms ease,
      background 120ms ease;
  }

  .tab-scroll:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
    max-width: 150px;
    padding: 5px 10px;
    border-radius: 8px;
    background: var(--bg-secondary);
    border: 1px solid transparent;
    color: var(--text-secondary);
    font-size: 12.5px;
    cursor: pointer;
    transition:
      background 120ms ease,
      color 120ms ease,
      border-color 120ms ease;
  }

  .tab:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .tab.active {
    background: var(--bg);
    border-color: var(--border);
    color: var(--text);
    font-weight: 600;
  }

  .tab.drop-target {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }

  .tab.dragging {
    opacity: 0.4;
  }

  /* Insertion indicator shown between tabs while dragging. */
  .tab-drop-line {
    align-self: stretch;
    width: 2px;
    margin: 3px 0;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
    flex: none;
  }

  .tab.editing {
    background: var(--bg);
    border-color: var(--accent);
    padding: 3px 8px;
  }

  .tab-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-input {
    width: 110px;
    background: none;
    border: none;
    outline: none;
    font-size: 12.5px;
    color: var(--text);
    padding: 2px 0;
  }

  .tab-add {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .tab-count {
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
    color: var(--text-disabled);
    background: var(--bg-secondary);
    border-radius: 5px;
    padding: 1px 4px;
    flex: none;
  }

  .tab.active .tab-count {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .tab-add:hover {
    background: var(--bg-hover);
    color: var(--text);
  }
</style>