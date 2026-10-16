<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { uiStore } from '$stores/uiStore.svelte';
  import {
    isValidDueDate,
    localDateStringAfterDays,
    nextDateString,
    previousDateString,
    todayLocalDateString,
  } from '$domain/taskValidation';
  import { t } from '$i18n';

  let {
    value,
    anchor,
    onPick,
    onClose,
  }: {
    value: string | null;
    anchor: DOMRect;
    onPick: (date: string | null) => void;
    onClose: () => void;
  } = $props();

  let popoverEl = $state<HTMLElement | undefined>(undefined);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  // Opening with no date starts on today, so the first click of "+" is not
  // needed and the field is never blank.
  let current = $state(untrack(() => value ?? todayLocalDateString()));
  let pos = $state(untrack(() => ({ left: anchor.right, top: anchor.bottom + 4 })));

  onMount(() => {
    inputEl?.focus();
    // Escape is handled centrally; this only puts the layer in the order.
    return uiStore.openLayer('due-date', onClose);
  });

  $effect(() => {
    function onDocumentClick(event: MouseEvent) {
      if (popoverEl && !popoverEl.contains(event.target as Node)) onClose();
    }
    document.addEventListener('mousedown', onDocumentClick);
    return () => document.removeEventListener('mousedown', onDocumentClick);
  });

  /**
   * The popover grows to the LEFT of the trigger (its right edge is pinned to
   * the trigger's right edge) so it opens away from the row's own buttons —
   * the comment and trash icons sit right of the date and must stay clickable.
   * If there is not enough room on the left, it falls back to growing rightwards.
   */
  $effect(() => {
    if (!popoverEl) return;
    const { width, height } = popoverEl.getBoundingClientRect();
    const fitsLeft = anchor.right - width >= 8;
    let left = fitsLeft ? anchor.right - width : anchor.left;
    let top = anchor.bottom + 6;
    if (top + height > window.innerHeight - 8) {
      top = Math.max(8, anchor.top - height - 6);
    }
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    pos = { left, top };
  });

  function apply() {
    onPick(current || null);
    onClose();
  }

  /** Wheel over the field steps the date exactly like the side buttons. */
  function onWheel(event: WheelEvent) {
    if (!isValidDueDate(current)) return;
    event.preventDefault();
    current = event.deltaY < 0 ? nextDateString(current) : previousDateString(current);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="due-popover due-popover-floating"
  bind:this={popoverEl}
  style="left: {pos.left}px; top: {pos.top}px;"
  role="dialog"
  aria-label={t('due.label')}
>
  <p class="popover-title">{t('due.label')}</p>

  <div class="due-stepper">
    <button
      class="step-button"
      onclick={() => (current = previousDateString(current))}
      aria-label={t('due.stepBack')}
      title={t('due.stepBack')}
    >
      −
    </button>

    <!-- Double click picks the date straight away, the wheel steps it. -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <input
      bind:this={inputEl}
      class="due-input"
      type="date"
      bind:value={current}
      onwheel={onWheel}
      ondblclick={apply}
      aria-label={t('due.label')}
      title={t('due.dblclickHint')}
      min="1970-01-01"
      max="2099-12-31"
    />

    <button
      class="step-button"
      onclick={() => (current = nextDateString(current))}
      aria-label={t('due.stepForward')}
      title={t('due.stepForward')}
    >
      +
    </button>
  </div>

  <div class="due-quick">
    <button
      class="quick-chip"
      class:selected={current === todayLocalDateString()}
      onclick={() => (current = todayLocalDateString())}
    >
      {t('due.today')}
    </button>
    <button
      class="quick-chip"
      class:selected={current === localDateStringAfterDays(1)}
      onclick={() => (current = localDateStringAfterDays(1))}
    >
      {t('due.tomorrow')}
    </button>
  </div>

  <div class="due-actions">
    <button
      class="link"
      disabled={value === null}
      onclick={() => {
        onPick(null);
        onClose();
      }}
    >
      {t('due.reset')}
    </button>
    <button
      class="link primary"
      disabled={!isValidDueDate(current) && current !== ''}
      onclick={apply}
    >
      {t('due.save')}
    </button>
  </div>
</div>

<style>
  .due-popover {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 196px;
  }

  .popover-title {
    margin: 0;
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  /* [ − ] [ date input ] [ + ] */
  .due-stepper {
    display: flex;
    align-items: stretch;
    gap: 4px;
  }

  .step-button {
    flex: none;
    width: 28px;
    padding: 0;
    font-size: 15px;
    line-height: 1;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 7px;
    cursor: default;
    transition:
      background 120ms ease,
      color 120ms ease,
      border-color 120ms ease;
  }

  .step-button:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  /* The field is the brightest surface in the popover: it is the control the
     whole panel is about. */
  .due-input {
    flex: 1 1 auto;
    min-width: 0;
    padding: 6px 8px;
    background: var(--input-bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    outline: none;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    color-scheme: inherit;
    box-shadow: 0 1px 2px rgba(15, 18, 25, 0.06);
  }

  .due-input:hover {
    border-color: var(--accent);
    background: var(--input-bg-hover);
  }

  .due-input:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>