<script lang="ts">
  import { untrack } from 'svelte';
  import { normalizeTitle } from '$domain/taskValidation';

  let {
    value,
    onSave,
    onCancel,
    ariaLabel,
  }: {
    value: string;
    onSave: (title: string) => void | Promise<void>;
    onCancel: () => void;
    ariaLabel: string;
  } = $props();

  // The editor starts from the current title; later edits live in `current`.
  let current = $state(untrack(() => value));
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  $effect(() => {
    inputEl?.focus();
    inputEl?.select();
  });

  async function submit() {
    const normalized = normalizeTitle(current);
    if (!normalized) return; // empty title is not saved
    await onSave(normalized);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      void submit();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      onCancel();
    }
  }
</script>

<input
  bind:this={inputEl}
  class="inline-edit"
  bind:value={current}
  aria-label={ariaLabel}
  maxlength="500"
  onkeydown={onKeydown}
  onblur={onCancel}
/>
