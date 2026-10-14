/** Pure validation helpers, mirrored in src-tauri/src/domain/validation.rs. */

/** Trims raw input and rejects empty titles. */
export function normalizeTitle(raw: string): string | null {
  const trimmed = raw.trim();
  return trimmed.length === 0 ? null : trimmed;
}

/** Strict YYYY-MM-DD calendar date validation (rejects 2025-02-30 etc.). */
export function isValidDueDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [year, month, day] = value.split('-').map(Number);
  if (month < 1 || month > 12 || day < 1 || day > 31) return false;
  const date = new Date(Date.UTC(year, month - 1, day));
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}

/** Today's calendar date in the user's local timezone (YYYY-MM-DD). */
export function todayLocalDateString(now: Date = new Date()): string {
  return toLocalDateString(now);
}

/** A calendar date `days` after today (YYYY-MM-DD), local time. */
export function localDateStringAfterDays(days: number, now: Date = new Date()): string {
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + days);
  return toLocalDateString(next);
}

function toLocalDateString(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/** Shifts a calendar date by `days`, without timezone drift. */
export function shiftDateString(value: string, days: number): string {
  const [year, month, day] = value.split('-').map(Number);
  return toLocalDateString(new Date(year, month - 1, day + days));
}

/**
 * One day later than `value`; an empty value means "pick today first", so the
 * + button always lands on a real date.
 */
export function nextDateString(value: string, now: Date = new Date()): string {
  if (value === '') return todayLocalDateString(now);
  return shiftDateString(value, 1);
}

/**
 * One day earlier than `value`, never going into the past: the earliest date
 * the stepper can produce is today.
 */
export function previousDateString(value: string, now: Date = new Date()): string {
  const today = todayLocalDateString(now);
  if (value === '') return today;
  const shifted = shiftDateString(value, -1);
  return shifted < today ? today : shifted;
}
