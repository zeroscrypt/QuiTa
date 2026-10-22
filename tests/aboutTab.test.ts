import { describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import SettingsPanel from '../src/components/SettingsPanel.svelte';

/**
 * «О программе» is a settings tab now, sharing its body with the native
 * About dialog. The tab itself must be reachable from the plain tab strip.
 */
describe('settings about tab', () => {
  it('lists the About tab among the settings tabs', () => {
    const body = render(SettingsPanel).body;
    // The strip always renders; the tab's body renders on opening.
    expect(body).toContain('settings-tab');
    expect(body).toContain('О программе');
  });
});
