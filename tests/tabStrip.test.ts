import { describe, expect, it } from 'vitest';

/**
 * Mirrors the wheel handling in `TabsBar.svelte`: a vertical wheel gesture
 * scrolls the tab strip sideways, and nothing happens when everything fits.
 */
export interface WheelInput {
  deltaX: number;
  deltaY: number;
}

export function stripScrollDelta({ deltaX, deltaY }: WheelInput): number {
  return Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY;
}

export function nextScrollLeft(current: number, delta: number, max: number): number {
  return Math.min(max, Math.max(0, current + delta));
}

/** Edge fades appear only when there is something to scroll towards. */
export function overflowEdges(scrollLeft: number, scrollWidth: number, clientWidth: number) {
  const max = scrollWidth - clientWidth;
  return {
    left: scrollLeft > 1,
    right: max > 1 && scrollLeft < max - 1,
  };
}

describe('strip wheel scrolling', () => {
  it('turns a vertical wheel gesture into horizontal movement', () => {
    expect(stripScrollDelta({ deltaX: 0, deltaY: 40 })).toBe(40);
    expect(stripScrollDelta({ deltaX: 0, deltaY: -40 })).toBe(-40);
  });

  it('keeps a trackpad horizontal gesture as is', () => {
    expect(stripScrollDelta({ deltaX: 25, deltaY: 3 })).toBe(25);
  });

  it('scrolls and clamps at both ends', () => {
    expect(nextScrollLeft(0, 60, 200)).toBe(60);
    expect(nextScrollLeft(180, 60, 200)).toBe(200);
    expect(nextScrollLeft(40, -60, 200)).toBe(0);
  });
});

describe('overflow edges', () => {
  it('shows nothing when all tabs fit', () => {
    expect(overflowEdges(0, 300, 400)).toEqual({ left: false, right: false });
  });

  it('shows the right fade while there is more to the right', () => {
    expect(overflowEdges(0, 600, 400)).toEqual({ left: false, right: true });
  });

  it('shows both fades in the middle', () => {
    expect(overflowEdges(100, 600, 400)).toEqual({ left: true, right: true });
  });

  it('hides the right fade at the very end', () => {
    expect(overflowEdges(200, 600, 400)).toEqual({ left: true, right: false });
  });
});
describe('edge arrows', () => {
  it('appear only when there is room to scroll that way', () => {
    // Nothing hidden: no arrows at all.
    expect(overflowEdges(0, 300, 400)).toEqual({ left: false, right: false });
    // Pinned to the right end: only the left arrow.
    expect(overflowEdges(200, 600, 400)).toEqual({ left: true, right: false });
    // Pinned to the left end: only the right arrow.
    expect(overflowEdges(0, 600, 400)).toEqual({ left: false, right: true });
    // In the middle: both.
    expect(overflowEdges(100, 600, 400)).toEqual({ left: true, right: true });
  });

  it('moves roughly one screen per click', () => {
    const step = Math.max(120, Math.round(400 * 0.7));
    expect(step).toBe(280);
    expect(nextScrollLeft(0, step, 900)).toBe(280);
    expect(nextScrollLeft(280, step, 900)).toBe(560);
  });
});
