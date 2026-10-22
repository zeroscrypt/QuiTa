import { checkForUpdate, installUpdate, relaunch, type UpdateInfo } from '$services/updateService';
import { settingsStore } from '$stores/settingsStore.svelte';
import { t } from '$i18n';

type State = 'idle' | 'checking' | 'available' | 'downloading' | 'error';

/** A day between automatic checks: enough to notice a release, unnoticeable. */
const CHECK_INTERVAL = 24 * 60 * 60 * 1000;

/** How often the guard tick wakes up to test the interval. */
const HOUR = 60 * 60 * 1000;

/**
 * The opt-in rule behind the settings toggle: auto mode turns a found release
 * into the standard install flow with no click. A check that found nothing
 * stays quiet either way, and with the toggle off the banner waits for the
 * click, as it always has.
 */
export function shouldAutoUpdate(info: UpdateInfo | null, autoUpdate: boolean): boolean {
  return info !== null && autoUpdate;
}

/**
 * The launch happens once per run, but the window may stay open for days —
 * a machine that sleeps at check time would miss its turn. The count starts
 * at the last completed check, so a manual check in «О программе» pushes the
 * next automatic one a full day forward instead of double-checking.
 */
export function isCheckDue(lastCheckedAt: number, now: number): boolean {
  return now - lastCheckedAt >= CHECK_INTERVAL;
}

/**
 * In-app updates.
 *
 * QuiTa is offline-first, so this is deliberately quiet: the check runs after
 * launch, then once a day while the window lives (the store keeps ticking even
 * when the window sits in the tray — that is precisely when a re-check is
 * useful), and on demand. Every failure is swallowed. A machine with no
 * network, or a repository with no releases yet, simply never shows the
 * banner — there is nothing the user could do about it anyway.
 */
class UpdateStore {
  state = $state<State>('idle');
  info = $state<UpdateInfo | null>(null);
  error = $state<string | null>(null);
  /** Bytes downloaded so far, for the progress line. */
  downloaded = $state(0);
  total = $state<number | null>(null);

  private lastCheckedAt = Date.now();
  private periodicTimer: ReturnType<typeof setInterval> | null = null;

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

  /**
   * Arms the daily re-check for the whole run of the window. Called once from
   * `onMount`, after the launch check — guarded by the timer handle, so a
   * second call (or a Vite HMR re-mount) cannot stack another interval.
   */
  startPeriodicChecks(): void {
    if (this.periodicTimer) return;
    this.periodicTimer = setInterval(() => void this.checkIfDue(), HOUR);
    // Waking from sleep also counts: the tick may have been clamped for days.
    document.addEventListener('visibilitychange', () => void this.checkIfDue());
  }

  private async checkIfDue(): Promise<void> {
    if (!isCheckDue(this.lastCheckedAt, Date.now())) return;
    if (this.state !== 'idle') return;
    await this.check();
  }

  async check(): Promise<void> {
    this.state = 'checking';
    this.error = null;
    const info = await checkForUpdate();
    this.info = info;
    this.state = info ? 'available' : 'idle';
    // A completed check is a check, whatever started it: the day counter
    // restarts here, so the About-tab button and the timer agree.
    this.lastCheckedAt = Date.now();
    // Opt-in auto mode: the release installs itself. The store class is not a
    // singleton in tests, so the reading goes through the real one — the
    // launched app always has settings initialised before the first check.
    if (shouldAutoUpdate(info, settingsStore.autoUpdate)) void this.update();
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