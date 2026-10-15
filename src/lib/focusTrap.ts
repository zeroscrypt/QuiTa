/**
 * Keeps keyboard focus inside a dialog while it is open.
 *
 * The dialogs carry `role="alertdialog" aria-modal="true"`, which promises
 * that the content outside is unavailable. Nothing made it so: the task list
 * behind stayed mounted and fully focusable, so Tab walked out of the dialog
 * and Enter there toggled tasks the user was not looking at. And when the
 * dialog unmounted, focus fell through to `<body>` — a keyboard user lost their
 * place in the list entirely.
 *
 * Two halves: Tab is wrapped around the focusable elements inside, and the
 * element focused before the dialog opened is refocused afterwards.
 */

const FOCUSABLE = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

function focusable(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
}

export function trapFocus(node: HTMLElement) {
  const previous = document.activeElement as HTMLElement | null;

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'Tab') return;
    const items = focusable(node);
    if (items.length === 0) {
      event.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const at = document.activeElement as HTMLElement | null;

    if (event.shiftKey && (at === first || !node.contains(at))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && at === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', onKeydown);

  return {
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      // Returning focus is what stops a closed dialog from dumping the user
      // back at the top of the document.
      if (previous && document.contains(previous)) previous.focus();
    },
  };
}