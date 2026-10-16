import type { TaskId } from "$lib/types";

/**
 * Which tab a keyboard shortcut selects.
 *
 * Digits stand for a position in the strip the user can see, so a number past
 * the last tab selects nothing rather than wrapping round. Wrapping would make
 * ⌘7 jump somewhere else every time a tab is closed, and the user has no way to
 * predict it from what is on screen.
 */
export function tabForDigit(tabs: readonly { id: TaskId }[], digit: number): TaskId | null {
  if (!Number.isInteger(digit) || digit < 1 || digit > 9) return null;
  return tabs[digit - 1]?.id ?? null;
}

/**
 * Whether a key event is a tab shortcut on this platform.
 *
 * Every app with tabs uses ⌘1..9 on macOS and Ctrl+1..9 on Windows and Linux.
 * Both modifiers are accepted everywhere rather than branching on the
 * operating system: they mean nothing else in this window, and detecting the
 * platform to reject one of them would add code for no gain.
 *
 * Alt and Shift are excluded on purpose — ⌘⇧1 and ⌥⌘1 belong to whoever bound
 * them, not to us.
 */
export function isTabShortcut(event: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}): boolean {
  if (!event.metaKey && !event.ctrlKey) return false;
  if (event.altKey || event.shiftKey) return false;
  return /^[1-9]$/.test(event.key);
}