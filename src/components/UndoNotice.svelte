<script lang="ts">
  import { uiStore } from '$stores/uiStore.svelte';
  import { t } from '$i18n';

  const notice = $derived(uiStore.undoNotice);
  /** 0…1 — the ring empties as the seconds run out. */
  const progress = $derived(
    notice ? Math.max(0, notice.secondsLeft / notice.totalSeconds) : 0,
  );
</script>

{#if notice}
  <div class="undo-notice" role="status">
    <!-- Ring around the countdown: the arc shortens as time runs out. -->
    <span class="ring" aria-hidden="true">
      <svg viewBox="0 0 36 36" width="34" height="34">
        <circle class="ring-track" cx="18" cy="18" r="16" pathLength="100" />
        <circle
          class="ring-value"
          cx="18"
          cy="18"
          r="16"
          pathLength="100"
          stroke-dasharray="100"
          stroke-dashoffset={100 - progress * 100}
        />
      </svg>
      <span class="ring-number">{notice.secondsLeft}</span>
    </span>

    <span class="undo-text">
      {notice.kind === 'flatten' ? t('undo.flattenTitle') : t('undo.tabTitle')}
      <span class="undo-detail">
        {notice.kind === 'flatten'
          ? t('undo.flattenDetail', { count: String(notice.flattenedCount) })
          : t('undo.tabDetail', { count: String(notice.taskCount) })}
      </span>
    </span>

    <button class="undo-button" onclick={() => void uiStore.undo()}>
      {t('undo.action')}
    </button>
  </div>
{/if}

<style>
  .undo-notice {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    color: var(--danger);
    background: var(--danger-soft);
    border-top: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
    flex: none;
    animation: notice-in 160ms ease;
  }

  @keyframes notice-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* The countdown ring: `pathLength=100` makes the maths radius-independent,
     so the arc is simply 100 units long and shrinks as seconds pass. */
  .ring {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    flex: none;
  }

  .ring svg {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
  }

  .ring-track,
  .ring-value {
    fill: none;
    stroke-width: 2.5;
    stroke-linecap: round;
  }

  .ring-track {
    stroke: color-mix(in srgb, var(--danger) 20%, transparent);
  }

  .ring-value {
    stroke: var(--danger);
    transition: stroke-dashoffset 950ms linear;
  }

  .ring-number {
    font-size: 12px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    color: var(--danger);
  }

  .undo-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1 1 auto;
    font-size: 12.5px;
    font-weight: 550;
    color: var(--danger);
  }

  .undo-detail {
    font-size: 11px;
    font-weight: 400;
    color: color-mix(in srgb, var(--danger) 75%, var(--text));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .undo-button {
    flex: none;
    padding: 5px 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--danger);
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--danger) 45%, transparent);
    border-radius: 7px;
    cursor: default;
    transition:
      background 120ms ease,
      border-color 120ms ease;
  }

  .undo-button:hover {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border-color: var(--danger);
  }

  @media (prefers-reduced-motion: reduce) {
    .undo-notice {
      animation: none;
    }

    .ring-value {
      transition: none;
    }
  }
</style>