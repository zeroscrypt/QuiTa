import { getAppInfo, type AppInfo } from '$services/appService';

/**
 * Product metadata from the binary: version, author, repository and the
 * global shortcut as it is registered on this machine.
 *
 * Shared by the About dialog and the settings shortcut row so the two cannot
 * disagree — the shortcut used to be written into the settings markup by hand
 * and drifted from what the app actually registered.
 */
class AppStore {
  info = $state<AppInfo | null>(null);

  /**
   * Input limits, straight from the rules the repository enforces.
   *
   * The fallbacks only matter for the moment before the first response: typing
   * faster than the round trip should not silently lift the limit.
   */
  limit = $derived({
    title: this.info?.limits.title ?? 500,
    tabName: this.info?.limits.tabName ?? 40,
    comment: this.info?.limits.comment ?? 2000,
  });

  private requested = false;

  async load(): Promise<void> {
    // Both callers mount independently; one fetch is enough.
    if (this.requested) return;
    this.requested = true;
    try {
      this.info = await getAppInfo();
    } catch {
      // Metadata is cosmetic: a failure leaves the placeholders in place.
      this.info = null;
    }
  }
}

export const appStore = new AppStore();