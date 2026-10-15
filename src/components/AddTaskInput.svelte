<script lang="ts">
  import { t } from '$i18n';
  import { onMount } from 'svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { appStore } from '$stores/appStore.svelte';
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
    // addRoot reports failure by returning false and raising a toast — it never
    // throws. The catch below therefore never ran, so the text cleared before
    // the request was simply gone when the request failed, which is the one
    // thing a storm-typing user cannot afford to lose.
    const added = await taskStore.addRoot(normalized ?? action.title);
    if (!added) {
      // The reason is in taskStore.error, where the toast already came from;
      // showing it next to the field as well is what makes it clear that the
      // text is still there and waiting.
      title = action.title;
      error = taskStore.error;
    }
    submitting = false;
    inputEl?.focus();
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
    maxlength={appStore.limit.title}
  />
</form>
{#if error}
  <div class="field-error" role="alert">{error}</div>
{/if}
