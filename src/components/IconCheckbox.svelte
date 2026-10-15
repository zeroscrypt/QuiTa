<script lang="ts">
  let {
    checked,
    partial = false,
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
    label: string;
    onToggle: () => void;
  } = $props();
</script>

<button
  class="checkbox"
  class:partial
  type="button"
  role="checkbox"
  aria-checked={partial ? 'mixed' : checked}
  aria-label={label}
  onclick={(event) => {
    event.stopPropagation();
    onToggle();
  }}
>
  <svg width="11" height="11" viewBox="0 0 12 12" aria-hidden="true">
    {#if partial}
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