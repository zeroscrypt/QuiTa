import { getSettings, updateSettings } from '$services/settingsService';
import { setLanguage, type Language } from '$i18n';

type Theme = 'system' | 'light' | 'dark';

class SettingsStore {
  theme = $state<Theme>('system');
  showCompleted = $state(false);
  cascadeComplete = $state(true);
  expandSubtasks = $state(true);
  showTabCounts = $state(true);
  newestFirst = $state(true);
  autoUpdate = $state(false);
  autoDeleteDays = $state<number | null>(null);
  trashLimit = $state(200);
  trashRetentionDays = $state<number | null>(null);
  language = $state<Language>('ru');
  loaded = $state(false);

  async init(): Promise<void> {
    try {
      const settings = await getSettings();
      this.theme = settings.theme;
      this.showCompleted = settings.showCompleted;
      this.cascadeComplete = settings.cascadeComplete ?? true;
      this.expandSubtasks = settings.expandSubtasks ?? true;
      this.showTabCounts = settings.showTabCounts ?? true;
      this.newestFirst = settings.newestFirst ?? true;
      this.autoUpdate = settings.autoUpdate ?? false;
      this.autoDeleteDays = settings.autoDeleteDays ?? null;
      this.trashLimit = settings.trashLimit ?? 200;
      this.trashRetentionDays = settings.trashRetentionDays ?? null;
      this.language = settings.language ?? 'ru';
    } catch {
      // Defaults stay; the app remains usable if settings cannot be read.
    }
    this.loaded = true;
    this.applyTheme();
    setLanguage(this.language);
    // Kept in a field so a re-initialisation (Vite HMR) detaches the old
    // listener instead of stacking another one.
    this.darkQuery?.removeEventListener('change', this.onSystemThemeChange);
    this.darkQuery = window.matchMedia('(prefers-color-scheme: dark)');
    this.darkQuery.addEventListener('change', this.onSystemThemeChange);
  }

  private darkQuery: MediaQueryList | null = null;
  private onSystemThemeChange = () => this.applyTheme();

  async setTheme(theme: Theme): Promise<void> {
    this.theme = theme;
    this.applyTheme();
    await this.persist();
  }

  async setShowCompleted(value: boolean): Promise<void> {
    this.showCompleted = value;
    await this.persist();
  }

  async setCascadeComplete(value: boolean): Promise<void> {
    this.cascadeComplete = value;
    await this.persist();
  }

  async setExpandSubtasks(value: boolean): Promise<void> {
    this.expandSubtasks = value;
    await this.persist();
  }

  async setShowTabCounts(value: boolean): Promise<void> {
    this.showTabCounts = value;
    await this.persist();
  }

  async setNewestFirst(value: boolean): Promise<void> {
    this.newestFirst = value;
    await this.persist();
  }

  /** `null` disables automatic cleanup of completed tasks. */
  async setAutoDeleteDays(days: number | null): Promise<void> {
    this.autoDeleteDays = days;
    await this.persist();
  }

  /** Zero turns the size rule off; 200 is the ceiling. */
  async setTrashLimit(limit: number): Promise<void> {
    this.trashLimit = Math.min(200, Math.max(0, Math.trunc(limit)));
    await this.persist();
  }

  /** Opt-in: a found release installs itself instead of waiting in the banner. */
  async setAutoUpdate(value: boolean): Promise<void> {
    this.autoUpdate = value;
    await this.persist();
  }

  /**
   * `null` keeps what is in the trash until the user says otherwise. The default
   * is `null` on purpose: a deletion nobody asked for must not follow from a
   * setting nobody filled in.
   */
  async setTrashRetentionDays(days: number | null): Promise<void> {
    this.trashRetentionDays = days;
    await this.persist();
  }

  async setLanguage(language: Language): Promise<void> {
    this.language = language;
    // `setLanguage` from the i18n module already updates `<html lang>`.
    setLanguage(language);
    await this.persist();
  }

  /**
   * Writes the whole snapshot. Calls are serialised: the UI updates state
   * immediately while the previous write may still be in flight, and two
   * overlapping `update_settings` calls could land out of order and leave a
   * stale value in the database.
   */
  private persistQueue: Promise<void> = Promise.resolve();

  private persist(): Promise<void> {
    const snapshot = {
      theme: this.theme,
      showCompleted: this.showCompleted,
      cascadeComplete: this.cascadeComplete,
      expandSubtasks: this.expandSubtasks,
      showTabCounts: this.showTabCounts,
      newestFirst: this.newestFirst,
      autoUpdate: this.autoUpdate,
      autoDeleteDays: this.autoDeleteDays,
      trashLimit: this.trashLimit,
      trashRetentionDays: this.trashRetentionDays,
      language: this.language,
    };
    this.persistQueue = this.persistQueue.then(async () => {
      try {
        await updateSettings(snapshot);
      } catch {
        // Local state already reflects the user's choice.
      }
    });
    return this.persistQueue;
  }

  private applyTheme(): void {
    const resolved =
      this.theme === 'system'
        ? window.matchMedia('(prefers-color-scheme: dark)').matches
          ? 'dark'
          : 'light'
        : this.theme;
    document.documentElement.dataset.theme = resolved;
  }
}

export const settingsStore = new SettingsStore();
