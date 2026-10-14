/**
 * Scroll maths for "jump to a row".
 *
 * `offsetTop` cannot be used here: the scroller is not a positioned
 * ancestor, so a row's `offsetTop` is measured from an ancestor further up
 * the tree and overshoots by the container's own page offset — which lands on
 * the wrong row. Measuring both rectangles in the same coordinate space does
 * not depend on the offset parent at all.
 */
export interface ScrollTarget {
  /** Current scroll offset of the container. */
  scrollTop: number;
  /** `getBoundingClientRect().top` of the scroll container. */
  scrollerTop: number;
  /** `getBoundingClientRect().top` of the row to reveal. */
  targetTop: number;
  /** Gap left above the row, in pixels. */
  inset?: number;
}

export function scrollTopForTarget({
  scrollTop,
  scrollerTop,
  targetTop,
  inset = 8,
}: ScrollTarget): number {
  const delta = targetTop - scrollerTop;
  return Math.max(0, Math.round(scrollTop + delta - inset));
}

/** Picks the row the "jump to newest" button must reveal. */
export function newestRowIndex(count: number, newestFirst: boolean): number | null {
  if (count <= 0) return null;
  return newestFirst ? 0 : count - 1;
}