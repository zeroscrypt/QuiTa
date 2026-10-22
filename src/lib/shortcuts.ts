/**
 * How the window shortcuts are spelled on the keyboard in front of the user.
 *
 * macOS labels its command key ⌘ and nobody labels it "Cmd"; every other
 * keyboard this app runs on labels it Ctrl, and a Russian Windows keyboard says
 * "Ctrl" too. The spelling follows the machine, taken from the backend, because
 * showing "⌘T" to someone pressing Ctrl+T is worse than showing nothing.
 */
export function modifierLabel(platform: string | null | undefined): string {
  return platform === "macos" ? "⌘" : "Ctrl";
}

/** One row of the shortcut list in the settings. */
export interface ShortcutRow {
  /** Keys as they are pressed, each on its own element. */
  keys: string[];
  /** i18n key for what the combination does. */
  labelKey: string;
}

/**
 * Every shortcut the app answers to, in the order a person meets them: the
 * global one first (it works from any application), then the window keys, then
 * the two keys people press without thinking.
 *
 * The global combination is passed in because the backend is where it is
 * registered: writing "⌘⇧Q" into the markup once showed Mac keys on a Windows
 * keyboard, where that key does not exist.
 */
export function shortcutRows(
  platform: string | null | undefined,
  globalShortcut: string,
): ShortcutRow[] {
  const mod = modifierLabel(platform);
  return [
    { keys: [globalShortcut], labelKey: "shortcuts.window" },
    { keys: [mod, "1…9"], labelKey: "shortcuts.switchTab" },
    { keys: [mod, "T"], labelKey: "shortcuts.newTab" },
    { keys: [mod, "F"], labelKey: "shortcuts.search" },
    { keys: [mod, "S"], labelKey: "shortcuts.settings" },
    { keys: [mod, "D"], labelKey: "shortcuts.markNotRelevant" },
    { keys: ["Enter"], labelKey: "shortcuts.create" },
    { keys: ["Esc"], labelKey: "shortcuts.closeLayer" },
  ].filter((row) => row.keys[0] !== "") as ShortcutRow[];
}