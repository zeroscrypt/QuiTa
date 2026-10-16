<script lang="ts">
  import { t } from '$i18n';
  import { onMount } from 'svelte';
  import { taskStore } from '$stores/taskStore.svelte';
  import { appStore } from '$stores/appStore.svelte';
  import { uiStore } from '$stores/uiStore.svelte';
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

  // Switching tabs with the keyboard ends with the caret here, so the next
  // thing typed becomes a task in the tab just selected. Without this the
  // shortcut would switch tabs and then leave the keyboard user hunting for
  // where to type.
  $effect(() => {
    if (uiStore.focusInputRequest > 0) focusInput();
  });

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
    // An open layer owns Escape, and it will put the caret back here when it
    // closes. Clearing the field as well would throw away half-typed text for
    // a keystroke the user meant for the popover above it.
    if (uiStore.layerCount > 0) return;
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
