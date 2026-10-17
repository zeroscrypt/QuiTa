import { describe, expect, it } from 'vitest';
import { modifierLabel, shortcutRows } from '../src/lib/shortcuts';

describe('modifierLabel', () => {
  it('spells the command key the way the keyboard in front of the user does', () => {
    // The mistake this exists to prevent: showing "⌘T" to someone whose only
    // Ctrl key is the one they are about to press.
    expect(modifierLabel('macos')).toBe('⌘');
    expect(modifierLabel('windows')).toBe('Ctrl');
    expect(modifierLabel('linux')).toBe('Ctrl');
  });

  it('falls back to Ctrl when the platform is not known yet', () => {
    // Before the first `app_info` arrives the list renders; a missing key must
    // not put a Mac symbol on a PC keyboard, and Ctrl is the safer guess
    // because it is also correct on a Mac with a full-size keyboard.
    expect(modifierLabel(null)).toBe('Ctrl');
    expect(modifierLabel(undefined)).toBe('Ctrl');
    expect(modifierLabel('')).toBe('Ctrl');
  });
});

describe('shortcutRows', () => {
  it('lists every shortcut the app answers to', () => {
    const rows = shortcutRows('macos', '⌘⇧Q');
    expect(rows.map((row) => row.labelKey)).toEqual([
      'shortcuts.window',
      'shortcuts.switchTab',
      'shortcuts.newTab',
      'shortcuts.search',
      'shortcuts.settings',
      'shortcuts.create',
      'shortcuts.closeLayer',
    ]);
  });

  it('puts the global shortcut first, spelled by the backend', () => {
    // The global combination is registered in Rust and differs per platform;
    // writing it into the markup once put "⌘⇧Q" on a Windows keyboard.
    const windows = shortcutRows('windows', 'Win+Alt+Q');
    expect(windows[0].keys).toEqual(['Win+Alt+Q']);
    const linux = shortcutRows('linux', 'Ctrl+Alt+Q');
    expect(linux[0].keys).toEqual(['Ctrl+Alt+Q']);
  });

  it('uses the platform modifier for the window keys and none for the rest', () => {
    const mac = shortcutRows('macos', '⌘⇧Q');
    expect(mac.find((r) => r.labelKey === 'shortcuts.newTab')?.keys).toEqual(['⌘', 'T']);
    expect(mac.find((r) => r.labelKey === 'shortcuts.search')?.keys).toEqual(['⌘', 'F']);
    expect(mac.find((r) => r.labelKey === 'shortcuts.settings')?.keys).toEqual(['⌘', 'S']);
    expect(mac.find((r) => r.labelKey === 'shortcuts.switchTab')?.keys).toEqual(['⌘', '1…9']);
    // Enter and Esc have no modifier, on any platform.
    expect(mac.find((r) => r.labelKey === 'shortcuts.create')?.keys).toEqual(['Enter']);
    expect(mac.find((r) => r.labelKey === 'shortcuts.closeLayer')?.keys).toEqual(['Esc']);

    const pc = shortcutRows('windows', 'Win+Alt+Q');
    expect(pc.find((r) => r.labelKey === 'shortcuts.newTab')?.keys).toEqual(['Ctrl', 'T']);
  });

  it('drops the global row when the backend has not answered yet', () => {
    // A row whose first key is empty renders as a lone "+", which is worse than
    // no row at all.
    const rows = shortcutRows('macos', '');
    expect(rows.every((row) => row.keys[0] !== '')).toBe(true);
    expect(rows.find((r) => r.labelKey === 'shortcuts.window')).toBeUndefined();
  });
});