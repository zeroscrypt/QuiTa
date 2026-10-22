/**
 * Natural due dates inside a task title.
 *
 * The storm input is for thoughts, not forms: «проверить диски за неделю»
 * must carry its own deadline instead of asking the user to open the picker
 * afterwards. The rule set here is deliberately narrow — it understands a
 * phrase only when the phrase is unambiguous, and everything else stays in
 * the title untouched.
 *
 * Recognized (case-insensitive, anywhere in the text):
 * - «сегодня», «завтра», «послезавтра» / «today», «tomorrow», «day after tomorrow»
 * - «за/через/в течение [N] дней|недель|месяцев|лет» / «in [N] days|weeks|months|years»,
 *   «in a week» — the numeral may be a digit (2), a word (две) or be omitted
 * - «в пятницу», «в пт» — the nearest day with that weekday, today counts;
 *   «в следующую пятницу» / «next friday» — the same weekday, but strictly after today
 * - «31.10», «31.10.2026», «2026-10-31» — calendar dates; an impossible one
 *   («31.02») is not recognized and leaves the title as it is
 *
 * Words like «неделю» or «пятницу» away from «за/через/в течение/в» never
 * move anything: bare nouns are words in a sentence, not deadlines.
 */

import {
  isValidDueDate,
  localDateStringAfterDays,
  todayLocalDateString,
} from './taskValidation';

export interface NaturalDueDate {
  /** Calendar date YYYY-MM-DD, or null when the title names no deadline. */
  dueDate: string | null;
  /** The same text with the matched phrase removed. */
  cleanedTitle: string;
}

/** Cyrillic-safe word edges: not a letter or digit before and after. */
const B = String.raw`(?<![\p{L}\p{N}])`;
const E = String.raw`(?![\p{L}\p{N}])`;
const F = 'giu';

/** Spoken numbers that may fill the numeral slot; an omitted numeral means one. */
const NUM_WORDS: Record<string, number> = {
  'один': 1, 'одну': 1, 'одна': 1, 'одного': 1, 'одной': 1,
  'два': 2, 'две': 2, 'двух': 2,
  'три': 3, 'трёх': 3,
  'четыре': 4, 'четырёх': 4,
  'пять': 5, 'пяти': 5,
  'шесть': 6, 'шести': 6,
  'семь': 7, 'семи': 7,
  'восемь': 8, 'восьми': 8,
  'девять': 9, 'девяти': 9,
  'десять': 10, 'десяти': 10,
  'one': 1, 'two': 2, 'three': 3, 'four': 4, 'five': 5,
  'six': 6, 'seven': 7, 'eight': 8, 'nine': 9, 'ten': 10,
};

const NUMERAL =
  String.raw`(\d{1,3}|один|одну|одна|одного|одной|две|два|двух|три|трёх|четыре|четырёх|пять|пяти|шесть|шести|семь|семи|восемь|восьми|девять|девяти|десять|десяти|one|two|three|four|five|six|seven|eight|nine|ten)?`;

const WEEKDAY_RU: Record<string, number> = {
  'понедельник': 1, 'вторник': 2, 'среду': 3, 'четверг': 4,
  'пятницу': 5, 'субботу': 6, 'воскресенье': 0,
};

const WEEKDAY_RU_SHORT: Record<string, number> = {
  'пн': 1, 'вт': 2, 'ср': 3, 'чт': 4, 'пт': 5, 'сб': 6, 'вс': 0,
};

const WEEKDAY_EN: Record<string, number> = {
  'monday': 1, 'tuesday': 2, 'wednesday': 3, 'thursday': 4,
  'friday': 5, 'saturday': 6, 'sunday': 0,
};

const WEEKDAY_EN_SHORT: Record<string, number> = {
  'mon': 1, 'tue': 2, 'wed': 3, 'thu': 4, 'fri': 5, 'sat': 6, 'sun': 0,
};

interface Candidate {
  /** Match offsets in the original title. */
  start: number;
  end: number;
  date: string;
}

/** A numeral: a digit, a known word, or nothing (which means one). */
function readCount(numeral: string | undefined): number | null {
  if (numeral === undefined || numeral === '') return 1;
  if (/^\d{1,3}$/.test(numeral)) return Number(numeral);
  return NUM_WORDS[numeral.toLowerCase()] ?? null;
}

type Unit = 'day' | 'week' | 'month' | 'year';

/** Turn a count and a unit into a calendar date from today. */
function resolveDuration(count: number, unit: Unit, now: Date): string {
  if (unit === 'day') return localDateStringAfterDays(count, now);
  if (unit === 'week') return localDateStringAfterDays(count * 7, now);
  return shiftByMonths(todayLocalDateString(now), unit === 'month' ? count : count * 12);
}

/** Names that are a fixed offset from today. */
function fixedDayCandidates(lower: string, now: Date): Candidate[] {
  const names: Array<[string, number]> = [
    ['сегодня', 0], ['today', 0],
    ['завтра', 1], ['tomorrow', 1],
    ['послезавтра', 2], ['day after tomorrow', 2],
  ];
  const out: Candidate[] = [];
  for (const [name, days] of names) {
    const re = new RegExp(`${B}${name}${E}`, F);
    for (const m of lower.matchAll(re)) {
      const at = m.index ?? 0;
      out.push({ start: at, end: at + m[0].length, date: localDateStringAfterDays(days, now) });
    }
  }
  return out;
}

/**
 * «за неделю», «через 2 недели», «в течение месяца», «in ten days».
 * The preposition is mandatory: bare «неделю» is a word in a sentence,
 * not a deadline.
 */
function durationCandidates(lower: string, now: Date): Candidate[] {
  const ruDays = String.raw`(?:день|дня|дню|днём|днями|дней|дни|дн)`;
  const ruWeeks = String.raw`(?:неделю|недели|неделях|недель|нед)`;
  const rules: Array<[RegExp, Unit]> = [
    [new RegExp(`${B}(?:за|через|в течение)\\s+(?:${NUMERAL})?\\s*${ruDays}${E}`, F), 'day'],
    [new RegExp(`${B}(?:за|через|в течение)\\s+(?:${NUMERAL})?\\s*${ruWeeks}${E}`, F), 'week'],
    [new RegExp(`${B}(?:за|через|в течение)\\s+(?:${NUMERAL})?\\s*меся[цасуе][а-яё]*${E}`, F), 'month'],
    [new RegExp(`${B}(?:за|через|в течение)\\s+(?:${NUMERAL})?\\s*(?:год[аоу]*|лет)${E}`, F), 'year'],
    [new RegExp(`${B}in\\s*${NUMERAL}\\s*days?${E}`, F), 'day'],
    [new RegExp(`${B}in\\s*${NUMERAL}\\s*days?${E}`, F), 'day'],
    [new RegExp(`${B}in\\s*${NUMERAL}\\s*weeks?${E}`, F), 'week'],
    [new RegExp(`${B}in\\s*${NUMERAL}\\s*months?${E}`, F), 'month'],
    [new RegExp(`${B}in\\s*${NUMERAL}\\s*years?${E}`, F), 'year'],
  ];
  const out: Candidate[] = [];
  for (const [re, unit] of rules) {
    for (const m of lower.matchAll(re)) {
      const at = m.index ?? 0;
      const count = readCount(m[1]);
      // 365 is the ceiling both for «через 400 недель»-style extremes and
      // « Version 2.5 »-style accidents: this is a task deadline, not a
      // calendar solver.
      if (count === null || count <= 0 || count > 365) continue;
      out.push({ start: at, end: at + m[0].length, date: resolveDuration(count, unit, now) });
    }
  }
  // «in a week» and friends: the unit itself carries the size, no numeral.
  const enOne = new RegExp(`${B}in a (day|week|month|year)${E}`, F);
  for (const m of lower.matchAll(enOne)) {
    const at = m.index ?? 0;
    const unit = m[1] as Unit;
    out.push({ start: at, end: at + m[0].length, date: resolveDuration(1, unit, now) });
  }
  return out;
}

/**
 * The day name and its table from a weekday match. «следующая»/«next» puts
 * the day strictly after today; without it, today counts when it already is
 * the named day. Short forms (пн, вт, mon, fri) only read with a preposition,
 * so a bare abbreviation stays a word.
 */
function weekdayRules(): Array<{ re: RegExp; table: Record<string, number>; full: boolean }> {
  const fullRu = Object.keys(WEEKDAY_RU).join('|');
  const shortRu = Object.keys(WEEKDAY_RU_SHORT).join('|');
  const fullEn = Object.keys(WEEKDAY_EN).join('|');
  const shortEn = Object.keys(WEEKDAY_EN_SHORT).join('|');
  return [
    // «в следующую пятницу», «в пятницу»
    { re: new RegExp(`${B}(?:в|во)\\s+(?:следующую\\s+)?(${fullRu})${E}`, F), table: WEEKDAY_RU, full: false },
    // bare «следующую пятницу»
    { re: new RegExp(`${B}следующую\\s+(${fullRu})${E}`, F), table: WEEKDAY_RU, full: false },
    // «в пт» — короткая форма только с предлогом
    { re: new RegExp(`${B}(?:в|во)\\s+(${shortRu})${E}`, F), table: WEEKDAY_RU_SHORT, full: false },
    // «on friday», «on fri»
    { re: new RegExp(`${B}on\\s+(?:next\\s+)?(${fullEn}|${shortEn})${E}`, F), table: { ...WEEKDAY_EN, ...WEEKDAY_EN_SHORT }, full: false },
    // «next friday» — без on
    { re: new RegExp(`${B}next\\s+(?:the\\s+)?(${fullEn}|${shortEn})${E}`, F), table: { ...WEEKDAY_EN, ...WEEKDAY_EN_SHORT }, full: false },
  ];
}

/** The nearest occurrence of a weekday offset, today included unless «next». */
function weekdayDate(
  match: string,
  name: string,
  table: Record<string, number>,
  now: Date,
): string | null {
  const day = table[name];
  if (day === undefined) return null;
  const strictlyAfter = /следую|next/i.test(match);
  const gap = (day - now.getDay() + 7) % 7;
  return localDateStringAfterDays(gap || (strictlyAfter ? 7 : 0), now);
}

function weekdayCandidates(lower: string, now: Date): Candidate[] {
  const out: Candidate[] = [];
  for (const { re, table } of weekdayRules()) {
    for (const m of lower.matchAll(re)) {
      const at = m.index ?? 0;
      const phrase = m[0];
      const date = weekdayDate(phrase, m[1] ?? '', table, now);
      if (date) out.push({ start: at, end: at + phrase.length, date });
    }
  }
  return out;
}

/**
 * «31.10», «31.10.2026», «2026-10-31» — calendar dates straight from the
 * title, never auto-corrected: an impossible date is not a date. A date
 * without a year lands on the nearest future occurrence. «2.5.6» — a version
 * number, not a date: the boundaries refuse a date glued to a dot from
 * either side, so a version or a fraction stays a number.
 */
function absoluteCandidates(title: string, now: Date): Candidate[] {
  const out: Candidate[] = [];
  // Dates sit next to punctuation and digits differently from words: a
  // leading or trailing dot («2.5.6»), a letter or a digit glued to the day
  // must all keep the candidate out.
  const DB = String.raw`(?<![\p{L}\p{N}.])`;
  const full = new RegExp(`${DB}(\\d{1,2})\\.(\\d{1,2})\\.(\\d{4})${E}`, F);
  for (const m of title.matchAll(full)) {
    const at = m.index ?? 0;
    // The text keeps what was written; the stored date is unambiguous ISO.
    const value = formatDate(Number(m[3]), Number(m[2]), Number(m[1]));
    if (isValidDueDate(value)) {
      out.push({ start: at, end: at + m[0].length, date: value });
    }
  }
  const iso = new RegExp(`${DB}(\\d{4}-\\d{2}-\\d{2})${E}`, F);
  for (const m of title.matchAll(iso)) {
    const at = m.index ?? 0;
    const value = m[1];
    if (isValidDueDate(value)) out.push({ start: at, end: at + value.length, date: value });
  }
  const dm = new RegExp(`${DB}(\\d{1,2})\\.(\\d{1,2})(?![.\\d])${E}`, F);
  for (const m of title.matchAll(dm)) {
    const at = m.index ?? 0;
    let date = formatDate(now.getFullYear(), Number(m[2]), Number(m[1]));
    // The nearest occurrence: a passed date rolls over to next year.
    if (!isValidDueDate(date) || date < todayLocalDateString(now)) {
      date = formatDate(now.getFullYear() + 1, Number(m[2]), Number(m[1]));
    }
    // Impossible both years (say 31.02) — not a date.
    if (isValidDueDate(date)) out.push({ start: at, end: at + m[0].length, date });
  }
  return out;
}

function formatDate(year: number, month: number, day: number): string {
  return `${year}-${pad(month)}-${pad(day)}`;
}

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

/** N calendar months ahead, clamped to the last real day of that month. */
function shiftByMonths(value: string, months: number): string {
  const [year, month, day] = value.split('-').map(Number);
  const target = new Date(year, month - 1 + months, 1);
  const last = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate();
  return toLocalDate(new Date(target.getFullYear(), target.getMonth(), Math.min(day, last)));
}

function toLocalDate(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/**
 * Pulls the leftmost, then longest recognized date phrase out of the title.
 * Everything unrecognized stays exactly as it was typed.
 */
export function extractNaturalDueDate(raw: string, now: Date = new Date()): NaturalDueDate {
  const lower = raw.toLowerCase();
  const candidates = [
    ...fixedDayCandidates(lower, now),
    ...durationCandidates(lower, now),
    ...weekdayCandidates(lower, now),
    ...absoluteCandidates(raw, now),
  ];
  if (candidates.length === 0) return { dueDate: null, cleanedTitle: raw };
  candidates.sort((a, b) => a.start - b.start || b.end - b.start - (a.end - a.start));
  const best = candidates[0];

  const cleaned = (raw.slice(0, best.start) + raw.slice(best.end))
    .replace(/\s{2,}/g, ' ')
    .trim();
  // The phrase that was the whole of the title leaves no task name behind:
  // nothing is invented, the text stays and no date is implied.
  if (!/\p{L}/u.test(cleaned)) return { dueDate: null, cleanedTitle: raw };
  return { dueDate: best.date, cleanedTitle: cleaned };
}
