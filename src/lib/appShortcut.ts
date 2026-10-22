/** What a key combination with the platform modifier asks the app to do. */
export type AppShortcut =
  | "newTab"
  | "focusSearch"
  | "settings"
  | "markNotRelevant";

interface KeyEventLike {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

/**
 * Which app-level action a key event asks for, or `null` if it asks for nothing.
 *
 * ⌘T, ⌘F, ⌘S and ⌘D on macOS, Ctrl+T, Ctrl+F, Ctrl+S and Ctrl+D on Windows and
 * Linux: the combinations every app with tabs already uses, so nobody has to
 * look them up. Both modifiers are accepted on every platform — they mean
 * nothing else in this window, and detecting the operating system to reject one
 * of them would carry code for no gain.
 *
 * Alt and Shift are excluded on purpose: ⌥⌘T and ⌘⇧F belong to whoever bound
 * them, not to us. The digit shortcuts live in `tabShortcut.ts` and cannot
 * collide — a key is either a digit or one of these four letters.
 */
export function appShortcut(event: KeyEventLike): AppShortcut | null {
  if (!event.metaKey && !event.ctrlKey) return null;
  if (event.altKey || event.shiftKey) return null;

  switch (event.key.toLowerCase()) {
    case "t":
      return "newTab";
    case "f":
      return "focusSearch";
    case "s":
      return "settings";
    case "d":
      return "markNotRelevant";
    default:
      return null;
  }
}
