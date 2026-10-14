import { describe, expect, it } from 'vitest';
import {
  isValidDueDate,
  localDateStringAfterDays,
  nextDateString,
  previousDateString,
  shiftDateString,
  todayLocalDateString,
} from '../src/domain/taskValidation';

const NOW = new Date(2026, 9, 14); // 14 October 2026, local time

describe('shiftDateString', () => {
  it('moves a date forward and backward without timezone drift', () => {
    expect(shiftDateString('2026-10-14', 1)).toBe('2026-10-15');
    expect(shiftDateString('2026-10-14', -1)).toBe('2026-10-13');
    expect(shiftDateString('2026-12-31', 1)).toBe('2027-01-01');
    expect(shiftDateString('2026-03-01', -1)).toBe('2026-02-28');
  });
});

describe('nextDateString', () => {
  it('picks today when nothing is selected yet', () => {
    expect(nextDateString('', NOW)).toBe('2026-10-14');
  });

  it('adds one day to an existing date', () => {
    expect(nextDateString('2026-10-20', NOW)).toBe('2026-10-21');
  });

  it('crosses a month boundary', () => {
    expect(nextDateString('2026-10-31', NOW)).toBe('2026-11-01');
  });
});

describe('previousDateString', () => {
  it('subtracts a day', () => {
    expect(previousDateString('2026-10-20', NOW)).toBe('2026-10-19');
  });

  it('starts at today when nothing is selected', () => {
    expect(previousDateString('', NOW)).toBe('2026-10-14');
  });

  // The stepper must never walk into the past.
  it('never goes before today', () => {
    expect(previousDateString('2026-10-15', NOW)).toBe('2026-10-14');
    expect(previousDateString('2026-10-14', NOW)).toBe('2026-10-14');
    expect(previousDateString('2026-10-01', NOW)).toBe('2026-10-14');
    expect(previousDateString('2020-01-01', NOW)).toBe('2026-10-14');
  });

  it('stays on today when repeated', () => {
    let value = nextDateString('', NOW);
    value = previousDateString(value, NOW);
    expect(value).toBe(todayLocalDateString(NOW));
  });
});

describe('relative day input', () => {
  it('maps a number of days to a calendar date', () => {
    expect(localDateStringAfterDays(0, NOW)).toBe('2026-10-14');
    expect(localDateStringAfterDays(7, NOW)).toBe('2026-10-21');
    expect(localDateStringAfterDays(365, NOW)).toBe('2027-10-14');
  });
});

describe('isValidDueDate', () => {
  it('accepts a calendar date and rejects an impossible one', () => {
    expect(isValidDueDate('2026-10-14')).toBe(true);
    expect(isValidDueDate('2026-02-30')).toBe(false);
    expect(isValidDueDate('2026-1-4')).toBe(false);
  });
});