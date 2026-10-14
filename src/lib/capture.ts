/**
 * Capture rules for the quick-entry fields.
 *
 * QuiTa has two of them — the root input at the top and the inline subtask
 * field inside a task — and they must behave identically, otherwise the user
 * has to learn two rhythms instead of one.
 *
 * The rhythm: type, Enter, type, Enter. Nothing closes, nothing is clicked,
 * and the caret stays where it is.
 */

/** What the field should do after a key or a pointer event. */
export type CaptureAction =
  /** Save this title and keep the field exactly as it is. */
  | { kind: 'add'; title: string }
  /** Nothing to save (empty field) or a save is already running. */
  | { kind: 'idle' }
  /** Throw away what is typed and close the field. */
  | { kind: 'cancel' }
  /** Save what is typed and close the field. */
  | { kind: 'save-and-close'; title: string };

function clean(raw: string): string {
  return raw.trim();
}

/**
 * Enter — the storm key. The field stays open and focused so the next task can
 * be typed immediately; this is the whole point of the gesture.
 */
export function onEnter(raw: string, busy = false): CaptureAction {
  const title = clean(raw);
  if (!title || busy) return { kind: 'idle' };
  return { kind: 'add', title };
}

/**
 * Escape — cancel. The root input has nowhere to close to, so there the same
 * rule just empties the field; the caller decides which of the two it is.
 */
export function onEscape(): CaptureAction {
  return { kind: 'cancel' };
}

/**
 * Leaving the field with the pointer finishes the gesture instead of losing
 * the text: whatever is typed is saved, then the field closes. Staying open
 * forever would make Escape the only way out, and a stray click is not intent.
 */
export function onLeave(raw: string, busy = false): CaptureAction {
  const title = clean(raw);
  if (!title || busy) return { kind: 'cancel' };
  return { kind: 'save-and-close', title };
}