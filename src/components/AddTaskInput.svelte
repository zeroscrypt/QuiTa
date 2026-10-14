<script lang="ts">
  import { t } from '$i18n';
  import { onMount } from 'svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { normalizeTitle } from '$domain/taskValidation';
  import { onEnter } from '$lib/capture';

  let title = $state('');
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let error = $state<string | null>(null);
  let submitting = $state(false);

  onMount(() => {
    inputEl?.focus();
    // Focused by the global shortcut / tray show event.
    window.addEventListener('quita:focus-add', focusInput);
    return () => window.removeEventListener('quita:focus-add', focusInput);
  });

  function focusInput() {
    inputEl?.focus();
  }

  function onInput() {
    if (error) error = null;
  }

  /**
   * Enter: add the task, clear the field, keep the caret. The storm rhythm is
   * the whole point of this input, so nothing here may steal the focus.
   */
  async function submit(event?: SubmitEvent) {
    event?.preventDefault();
    const action = onEnter(title, submitting);
    if (action.kind !== 'add') return;
    submitting = true;
    error = null;
    const normalized = normalizeTitle(action.title);
    title = '';
    try {
      await taskStore.addRoot(normalized ?? action.title);
    } catch (e) {
      // Keep the entered text so the user can retry without retyping.
      title = action.title;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      submitting = false;
      inputEl?.focus();
    }
  }

  /** Escape empties the field: the storm stops, the input stays where it is. */
  function clearField(event: KeyboardEvent) {
    event.preventDefault();
    if (!title) return;
    title = '';
    error = null;
  }
</script>

<form class="add-form" onsubmit={submit}>
  <input
    bind:this={inputEl}
    bind:value={title}
    oninput={onInput}
    onkeydown={(event) => {
      if (event.key === 'Escape') clearField(event);
    }}
    placeholder={t('add.placeholder')}
    aria-label={t('add.label')}
    aria-invalid={error !== null}
    maxlength="500"
  />
</form>
{#if error}
  <div class="field-error" role="alert">{error}</div>
{/if}
