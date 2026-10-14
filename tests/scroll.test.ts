import { describe, expect, it } from 'vitest';
import { newestRowIndex, scrollTopForTarget } from '../src/lib/scroll';

describe('scrollTopForTarget', () => {
  it('puts the row 8px below the top of the container', () => {
    // Container starts at y=100, the row is 40px lower and we are at 0.
    expect(
      scrollTopForTarget({ scrollTop: 0, scrollerTop: 100, targetTop: 140 }),
    ).toBe(32);
  });

  it('keeps the current scroll as the base, not the page offset', () => {
    // This is the regression: a row 500px into a list that is already
    // scrolled by 200 must land at 300, not 492.
    // delta = 500 → 200 + 500 - 8 = 692
    expect(
      scrollTopForTarget({ scrollTop: 200, scrollerTop: 100, targetTop: 600 }),
    ).toBe(692);
  });

  it('never scrolls above the start', () => {
    expect(
      scrollTopForTarget({ scrollTop: 0, scrollerTop: 100, targetTop: 95 }),
    ).toBe(0);
  });

  it('scrolls back up when the row is above the viewport', () => {
    // delta = 40 → 500 + 40 - 8 = 532
    expect(
      scrollTopForTarget({ scrollTop: 500, scrollerTop: 100, targetTop: 140 }),
    ).toBe(532);
  });

  it('honours a custom inset', () => {
    expect(
      scrollTopForTarget({ scrollTop: 0, scrollerTop: 0, targetTop: 50, inset: 0 }),
    ).toBe(50);
  });
});

describe('newestRowIndex', () => {
  it('returns the first row when new tasks go on top', () => {
    expect(newestRowIndex(5, true)).toBe(0);
  });

  it('returns the last row when new tasks go to the bottom', () => {
    expect(newestRowIndex(5, false)).toBe(4);
  });

  it('handles a single row and an empty list', () => {
    expect(newestRowIndex(1, true)).toBe(0);
    expect(newestRowIndex(1, false)).toBe(0);
    expect(newestRowIndex(0, true)).toBeNull();
  });
});