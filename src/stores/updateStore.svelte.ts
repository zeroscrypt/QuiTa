import { checkForUpdate, installUpdate, relaunch, type UpdateInfo } from '$services/updateService';
import { t } from '$i18n';

type State = 'idle' | 'checking' | 'available' | 'downloading' | 'error';

/**
 * In-app updates.
 *
 * QuiTa is offline-first, so this is deliberately quiet: the check runs once
 * after launch and on demand, and every failure is swallowed. A machine with
 * no network, or a repository with no releases yet, simply never shows the
 * banner — there is nothing the user could do about it anyway.
 */
class UpdateStore {
  state = $state<State>('idle');
  info = $state<UpdateInfo | null>(null);
  error = $state<string | null>(null);
  /** Bytes downloaded so far, for the progress line. */
  downloaded = $state(0);
  total = $state<number | null>(null);

  /** True only while a newer release is waiting and nothing is in flight. */
  get available(): boolean {
    return this.state === 'available' && this.info !== null;
  }

  get percent(): number | null {
    if (this.total === null || this.total <= 0) return null;
    return Math.min(100, Math.round((this.downloaded / this.total) * 100));
  }

  /**
   * Called once after launch. A check already in flight is not repeated: the
   * user may have opened the window from the tray while it was running.
   */
  async checkOnStart(): Promise<void> {
    if (this.state !== 'idle') return;
    await this.check();
  }

  async check(): Promise<void> {
    this.state = 'checking';
    this.error = null;
    const info = await checkForUpdate();
    this.info = info;
    this.state = info ? 'available' : 'idle';
  }

  /** Downloads, installs and restarts. */
  async update(): Promise<void> {
    if (!this.info) return;
    this.state = 'downloading';
    this.error = null;
    this.downloaded = 0;
    this.total = null;
    try {
      await installUpdate((downloaded, total) => {
        this.downloaded = downloaded;
        this.total = total;
      });
      // The new binary is in place; restarting is what makes it live.
      await relaunch();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.state = 'error';
    }
  }

  /** Closes the banner without installing: "обновлюсь позже". */
  dismiss(): void {
    this.state = 'idle';
    this.info = null;
    this.error = null;
  }

  message(key: string): string {
    return t(key);
  }
}

export const updateStore = new UpdateStore();