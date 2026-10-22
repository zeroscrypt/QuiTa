<script lang="ts">
  let {
    checked,
    partial = false,
    notRelevant = false,
    label,
    onToggle,
  }: {
    checked: boolean;
    /**
     * Completed, but subtasks are still open: shown as a dash.
     *
     * This also drives `aria-checked="mixed"`. Announcing a partly-done task as
     * plain checked is the state the whole `partial` concept exists to
     * distinguish — it has its own colour, its own icon and its own rule in
     * taskRules — and none of that reached the accessibility tree.
     */
    partial?: boolean;
    /**
     * Marked "не актуальная": shown as a red cross.
     *
     * The mark deliberately wins over `partial`: a red task with a dash would
     * read as an alarm with a footnote, and the other way around as done with
     * a caveat. The cross says the work was retired, not finished.
     */
    notRelevant?: boolean;
    label: string;
    onToggle: () => void;
  } = $props();

  const markedNotDone = $derived(notRelevant && !checked);
</script>

<button
  class="checkbox"
  class:partial={partial && !notRelevant}
  class:not-relevant={notRelevant}
  type="button"
  role="checkbox"
  aria-checked={partial && !notRelevant ? 'mixed' : checked}
  aria-label={label}
  onclick={(event) => {
    event.stopPropagation();
    onToggle();
  }}
>
  <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
    {#if notRelevant}
      <!-- Не актуальная: a cross, red on an open box or white on a red
           fill when the task is closed both ways. -->
      <path
        d="M2.5 2.5l7 7M9.5 2.5l-7 7"
        fill="none"
        stroke={markedNotDone ? 'var(--danger)' : '#fff'}
        stroke-width="1.9"
        stroke-linecap="round"
      />
    {:else if partial}
      <!-- Partially done: a dash, not a check. -->
      <path
        d="M2.6 6h6.8"
        fill="none"
        stroke="#fff"
        stroke-width="1.9"
        stroke-linecap="round"
      />
    {:else}
      <path
        d="M2 6.2 4.8 9 10 3.4"
        fill="none"
        stroke="#fff"
        stroke-width="1.9"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    {/if}
  </svg>
</button>
