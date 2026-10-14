<script lang="ts">
  let { text }: { text: string } = $props();

  let dotEl = $state<HTMLButtonElement | undefined>(undefined);
  let tipEl = $state<HTMLElement | undefined>(undefined);
  let open = $state(false);
  /** Viewport coordinates: the tooltip is `position: fixed` so the scrolling
      settings body cannot clip it. */
  let pos = $state({ left: 0, top: 0 });

  const TIP_WIDTH = 260;
  const GAP = 8;
  const MARGIN = 8;

  function show() {
    if (!dotEl) return;
    const rect = dotEl.getBoundingClientRect();
    // Measure after the first paint so the real width is known.
    open = true;
    const width = tipEl?.offsetWidth ?? TIP_WIDTH;
    const height = tipEl?.offsetHeight ?? 0;
    let left = rect.left + rect.width / 2 - width / 2;
    left = Math.max(MARGIN, Math.min(left, window.innerWidth - width - MARGIN));
    // Prefer above the dot; flip below when there is no room.
    let top = rect.top - height - GAP;
    if (top < MARGIN) top = rect.bottom + GAP;
    pos = { left, top };
  }

  function hide() {
    open = false;
  }
</script>

<button
  bind:this={dotEl}
  class="info-dot"
  type="button"
  aria-label={text}
  aria-expanded={open}
  onmouseenter={show}
  onmouseleave={hide}
  onfocus={show}
  onblur={hide}
  onclick={(e) => {
    e.preventDefault();
    show();
  }}
>
  <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
    <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.8" />
    <path d="M12 10.5v6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
    <circle cx="12" cy="7.2" r="1.15" fill="currentColor" />
  </svg>
</button>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={tipEl}
    class="info-tip"
    role="tooltip"
    style="left: {pos.left}px; top: {pos.top}px;"
  >
    {text}
  </div>
{/if}

<style>
  .info-dot {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    flex: none;
    color: var(--text-disabled);
    background: transparent;
    border: none;
    border-radius: 50%;
    cursor: help;
    transition: color 120ms ease;
  }

  .info-dot:hover,
  .info-dot:focus-visible {
    color: var(--accent);
  }

  /* Fixed positioning keeps the tooltip outside the scrolling settings body,
     which would otherwise cut it off, and the z-index lifts it above every
     panel and dialog in the app. */
  .info-tip {
    position: fixed;
    z-index: 10000;
    width: max-content;
    max-width: 260px;
    padding: 7px 9px;
    font-size: 11.5px;
    line-height: 1.45;
    font-weight: 400;
    /* The hint sits inside an uppercase heading in some places: reset the
       inherited casing so the explanation reads as a sentence. */
    text-transform: none;
    letter-spacing: normal;
    text-align: left;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow-menu);
    pointer-events: none;
    animation: tip-in 110ms ease;
  }

  @keyframes tip-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .info-tip {
      animation: none;
    }
  }
</style>