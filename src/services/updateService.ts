import { check } from '@tauri-apps/plugin-updater';

export interface UpdateInfo {
  version: string;
  /** Release notes, as published with the release. */
  notes: string;
  /** When the release was published, RFC 3339. */
  date: string;
}

/**
 * Looks for a newer release.
 *
 * Resolves to `null` when there is nothing newer, and also when the check
 * could not be made at all: an offline machine or a repository without
 * releases must not show an error the user cannot act on.
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  try {
    const update = await check();
    if (!update) return null;
    return {
      version: update.version,
      notes: update.body ?? '',
      date: update.date ?? '',
    };
  } catch {
    return null;
  }
}

/**
 * Downloads the release, applies it and restarts the app.
 *
 * The download is streamed so the caller can show progress; `onDone` fires
 * when the new binary is in place. Throws on a failed download or a failed
 * signature check — the caller decides whether that deserves a message.
 */
export async function installUpdate(
  onProgress: (downloaded: number, total: number | null) => void,
): Promise<string> {
  const update = await check();
  if (!update) throw new Error('no-update');

  let downloaded = 0;
  let total: number | null = null;

  await update.downloadAndInstall((event) => {
    switch (event.event) {
      case 'Started':
        total = event.data.contentLength ?? null;
        downloaded = 0;
        break;
      case 'Progress':
        downloaded += event.data.chunkLength;
        break;
      case 'Finished':
        downloaded = total ?? downloaded;
        break;
    }
    onProgress(downloaded, total);
  });

  return update.version;
}

/** Restarts the app into the freshly installed version. */
export async function relaunch(): Promise<void> {
  const { relaunch: relaunchApp } = await import('@tauri-apps/plugin-process');
  await relaunchApp();
}