import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'svelte/server';
import AboutBody from '../src/components/AboutBody.svelte';
import { appStore } from '../src/stores/appStore.svelte';
import { updateStore } from '../src/stores/updateStore.svelte';

/**
 * The body behind «О программе» — version, storage, repository and the
 * update states — is shared by the native About dialog and the settings tab,
 * so what one shows, both show.
 */
describe('about body', () => {
  afterEach(() => {
    // The singleton holds out across tests; each test leaves it quiet again.
    updateStore.dismiss();
    appStore.info = null;
  });

  it('names the version even before the app info arrives', () => {
    const body = render(AboutBody).body;
    expect(body).toContain('Версия');
    expect(body).toContain('—'); // unknown version stays unknown
    expect(body).toContain('MIT');
    // A quiet store shows no update machinery.
    expect(body).not.toContain('about-update');
  });

  it('shows the found release with the install action', () => {
    updateStore.state = 'available';
    updateStore.info = { version: '9.9.9', notes: '', date: '' };
    const body = render(AboutBody).body;
    expect(body).toContain('about-update'); // the strip exists
    expect(body).toContain('Доступна новая версия 9.9.9');
    expect(body).toContain('Обновить');
  });
});
