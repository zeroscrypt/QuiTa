import { describe, expect, it } from 'vitest';
import { updateStore, shouldAutoUpdate } from '../src/stores/updateStore.svelte';

/**
 * The banner is the only part of the app that talks to the network. Two
 * properties matter and are easy to break:
 *
 *  - it never appears without a real, newer release;
 *  - a failed check is silent, because an offline machine has nothing to act
 *    on and an error strip above the tabs would be noise.
 */
describe('update banner state', () => {
  it('starts hidden and quiet', () => {
    const store = new (updateStore.constructor as new () => typeof updateStore)();
    expect(store.state).toBe('idle');
    expect(store.available).toBe(false);
    expect(store.info).toBeNull();
    expect(store.error).toBeNull();
  });

  it('is only "available" while a release is waiting', () => {
    const store = new (updateStore.constructor as new () => typeof updateStore)();
    store.state = 'available';
    store.info = { version: '1.1.4', notes: '', date: '' };
    expect(store.available).toBe(true);

    // Downloading and erroring both hide the "new version" wording.
    store.state = 'downloading';
    expect(store.available).toBe(false);
    store.state = 'error';
    expect(store.available).toBe(false);
  });

  it('computes progress only when the total size is known', () => {
    const store = new (updateStore.constructor as new () => typeof updateStore)();
    store.downloaded = 512;
    store.total = null;
    expect(store.percent).toBeNull();

    store.total = 1024;
    expect(store.percent).toBe(50);

    store.downloaded = 2048;
    expect(store.percent).toBe(100);
  });

  it('forgets everything when dismissed', () => {
    const store = new (updateStore.constructor as new () => typeof updateStore)();
    store.state = 'available';
    store.info = { version: '1.1.4', notes: 'notes', date: '' };
    store.error = 'boom';

    store.dismiss();

    expect(store.state).toBe('idle');
    expect(store.info).toBeNull();
    expect(store.error).toBeNull();
  });

  it('auto mode needs a release and the opt-in together', () => {
    const info = { version: '1.2.0', notes: '', date: '' };
    // Nothing found: quiet, with or without the toggle.
    expect(shouldAutoUpdate(null, true)).toBe(false);
    // A release waits, but the toggle is off: the banner decides.
    expect(shouldAutoUpdate(info, false)).toBe(false);
    // The combination the toggle exists for.
    expect(shouldAutoUpdate(info, true)).toBe(true);
  });
});
