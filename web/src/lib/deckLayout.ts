/**
 * Deck layout geometry and packing.
 *
 * Pure math, no DOM and no framework. Everything here is a function of a
 * container size and a grid spec, which is what makes positions
 * *precalculable*: the component measures its container once, derives every
 * cell rectangle up front, and then absolutely positions widgets. Dragging and
 * resizing become arithmetic rather than reflow, and a widget never lands
 * somewhere the math did not predict.
 *
 * The model is a Stream Deck rather than a document flow:
 *
 * - A page is a fixed `columns x rows` grid of keys.
 * - Every widget owns an explicit `{page, x, y, w, h}` in **cell units**.
 * - Nothing auto-flows, so there are no surprise gaps when a widget changes
 *   size, and a layout looks identical on every screen width.
 */

/** A widget's footprint, in cell units. */
export interface CellRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** A widget's footprint plus the page it lives on. */
export interface Placement extends CellRect {
  page: number;
}

/** Absolute pixel box, ready for `style.transform` / `left`+`top`. */
export interface PixelRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** The shape of a page. */
export interface GridSpec {
  /** Cells across one page. */
  columns: number;
  /** Cells down one page. */
  rows: number;
  /** Pixels between adjacent cells. */
  gap: number;
  /** Pixels between the outermost cells and the page frame. */
  padding: number;
  /**
   * `cellWidth / cellHeight`. 1 gives square keys like real deck hardware;
   * above 1 gives landscape cells, which suits widgets carrying text.
   */
  aspect: number;
}

/** Everything the renderer needs, derived once per container measurement. */
export interface Geometry extends GridSpec {
  cellWidth: number;
  cellHeight: number;
  /** Rendered size of one page, including padding. */
  pageWidth: number;
  pageHeight: number;
  /**
   * Horizontal offset when the grid is narrower than its container, which
   * happens whenever a height limit forces smaller cells. Keeps the deck
   * centred instead of hugging the left edge.
   */
  offsetX: number;
}

export const DEFAULT_GRID: GridSpec = {
  columns: 4,
  rows: 3,
  gap: 12,
  padding: 12,
  aspect: 1.35,
};

/** Smallest cell we will render before giving up on fitting the height. */
const MIN_CELL_WIDTH = 72;

function trackSize(available: number, count: number, gap: number, padding: number): number {
  if (count <= 0) return 0;
  return (available - padding * 2 - gap * (count - 1)) / count;
}

/**
 * Derive cell and page dimensions from the space available.
 *
 * Width is the primary constraint. When `maxHeight` is supplied and the
 * resulting page would overflow it, cells shrink to fit *while preserving
 * `aspect`* — the alternative, squashing cells vertically, makes a deck look
 * broken at exactly the moment space is tight. The grid is then centred
 * horizontally via {@link Geometry.offsetX}.
 */
export function computeGeometry(
  containerWidth: number,
  spec: GridSpec = DEFAULT_GRID,
  maxHeight?: number,
): Geometry {
  const columns = Math.max(1, Math.floor(spec.columns));
  const rows = Math.max(1, Math.floor(spec.rows));
  const aspect = spec.aspect > 0 ? spec.aspect : 1;
  const { gap, padding } = spec;

  let cellWidth = Math.max(0, trackSize(containerWidth, columns, gap, padding));
  let cellHeight = cellWidth / aspect;

  if (maxHeight && maxHeight > 0) {
    const heightBound = Math.max(0, trackSize(maxHeight, rows, gap, padding));
    if (heightBound < cellHeight) {
      // Re-derive width from the height bound so the aspect ratio survives.
      const scaledWidth = heightBound * aspect;
      if (scaledWidth >= MIN_CELL_WIDTH) {
        cellHeight = heightBound;
        cellWidth = scaledWidth;
      }
      // Below MIN_CELL_WIDTH we keep the width-derived size and let the page
      // scroll: unreadably small keys are worse than a scrollbar.
    }
  }

  const pageWidth = cellWidth * columns + gap * (columns - 1) + padding * 2;
  const pageHeight = cellHeight * rows + gap * (rows - 1) + padding * 2;

  return {
    columns,
    rows,
    gap,
    padding,
    aspect,
    cellWidth,
    cellHeight,
    pageWidth,
    pageHeight,
    offsetX: Math.max(0, (containerWidth - pageWidth) / 2),
  };
}

/** Exact pixel box for a cell rectangle. */
export function rectToPixels(rect: CellRect, geom: Geometry): PixelRect {
  const { cellWidth, cellHeight, gap, padding, offsetX } = geom;
  return {
    left: offsetX + padding + rect.x * (cellWidth + gap),
    top: padding + rect.y * (cellHeight + gap),
    // Spanned cells reclaim the gaps they cover, so a 2-wide widget is exactly
    // as wide as two keys plus the gutter between them.
    width: rect.w * cellWidth + (rect.w - 1) * gap,
    height: rect.h * cellHeight + (rect.h - 1) * gap,
  };
}

/**
 * Which cell a pixel lands in.
 *
 * Rounds to the nearest cell boundary rather than flooring, so a widget
 * dragged just past the halfway point snaps forward the way a user expects.
 * The result is deliberately unclamped — callers decide whether an
 * out-of-range cell means "reject" or "clamp".
 */
export function pixelsToCell(px: number, py: number, geom: Geometry): { x: number; y: number } {
  const { cellWidth, cellHeight, gap, padding, offsetX } = geom;
  const strideX = cellWidth + gap;
  const strideY = cellHeight + gap;
  return {
    x: strideX > 0 ? Math.round((px - offsetX - padding) / strideX) : 0,
    y: strideY > 0 ? Math.round((py - padding) / strideY) : 0,
  };
}

/** How many cells a pixel distance spans, at minimum 1. */
export function pixelsToSpan(size: number, cell: number, gap: number): number {
  const stride = cell + gap;
  if (stride <= 0) return 1;
  return Math.max(1, Math.round((size + gap) / stride));
}

/** Keep a rectangle inside the page, preserving its size where possible. */
export function clampRect(rect: CellRect, spec: GridSpec): CellRect {
  const w = Math.min(Math.max(1, rect.w), spec.columns);
  const h = Math.min(Math.max(1, rect.h), spec.rows);
  return {
    w,
    h,
    x: Math.min(Math.max(0, rect.x), spec.columns - w),
    y: Math.min(Math.max(0, rect.y), spec.rows - h),
  };
}

/** Whether two rectangles share any cell. */
export function rectsOverlap(a: CellRect, b: CellRect): boolean {
  return (
    a.x < b.x + b.w &&
    b.x < a.x + a.w &&
    a.y < b.y + b.h &&
    b.y < a.y + a.h
  );
}

/** Whether `rect` fits on the page without touching any of `occupied`. */
export function fits(rect: CellRect, occupied: CellRect[], spec: GridSpec): boolean {
  if (rect.x < 0 || rect.y < 0) return false;
  if (rect.x + rect.w > spec.columns) return false;
  if (rect.y + rect.h > spec.rows) return false;
  return !occupied.some((other) => rectsOverlap(rect, other));
}

/**
 * First free slot for a `w x h` widget, scanning left-to-right, top-to-bottom.
 *
 * Returns `null` when the page is too full, which is the signal to start a new
 * page rather than to overlap something.
 */
export function findFreeSlot(
  occupied: CellRect[],
  w: number,
  h: number,
  spec: GridSpec,
): { x: number; y: number } | null {
  const width = Math.min(Math.max(1, w), spec.columns);
  const height = Math.min(Math.max(1, h), spec.rows);

  for (let y = 0; y <= spec.rows - height; y++) {
    for (let x = 0; x <= spec.columns - width; x++) {
      if (fits({ x, y, w: width, h: height }, occupied, spec)) {
        return { x, y };
      }
    }
  }
  return null;
}

/** The minimum a widget carries into the layout engine. */
export interface PlaceableWidget {
  id: string;
  /** Preferred footprint. Falls back to 1x1. */
  w?: number;
  h?: number;
  /** Explicit position; absent means "place me somewhere sensible". */
  x?: number;
  y?: number;
  page?: number;
}

/**
 * Assign a concrete `{page, x, y, w, h}` to every widget.
 *
 * Widgets that already have a valid, non-overlapping position keep it — moving
 * something the user deliberately placed is worse than leaving a gap. Anything
 * unplaced, off-page, or colliding is packed into the first slot that fits,
 * spilling onto a new page when the current one fills.
 *
 * This is also the migration path from the old flow layout, where widgets had
 * only `colSpan`/`rowSpan` and no coordinates at all.
 */
export function normalizeLayout(
  widgets: PlaceableWidget[],
  spec: GridSpec = DEFAULT_GRID,
): Map<string, Placement> {
  const result = new Map<string, Placement>();
  const byPage = new Map<number, CellRect[]>();

  const occupantsOf = (page: number): CellRect[] => {
    let list = byPage.get(page);
    if (!list) {
      list = [];
      byPage.set(page, list);
    }
    return list;
  };

  const size = (widget: PlaceableWidget) => ({
    w: Math.min(Math.max(1, Math.floor(widget.w ?? 1)), spec.columns),
    h: Math.min(Math.max(1, Math.floor(widget.h ?? 1)), spec.rows),
  });

  // Pass 1 — honour valid explicit positions, so packing never displaces a
  // widget the user placed by hand.
  const unplaced: PlaceableWidget[] = [];
  for (const widget of widgets) {
    const { w, h } = size(widget);
    const hasPosition =
      Number.isFinite(widget.x) && Number.isFinite(widget.y) && Number.isFinite(widget.page);

    if (!hasPosition) {
      unplaced.push(widget);
      continue;
    }

    const page = Math.max(0, Math.floor(widget.page as number));
    const rect: CellRect = { x: Math.floor(widget.x as number), y: Math.floor(widget.y as number), w, h };

    if (fits(rect, occupantsOf(page), spec)) {
      occupantsOf(page).push(rect);
      result.set(widget.id, { ...rect, page });
    } else {
      unplaced.push(widget);
    }
  }

  // Pass 2 — pack whatever is left into the earliest slot available.
  for (const widget of unplaced) {
    const { w, h } = size(widget);
    let page = 0;
    let slot = findFreeSlot(occupantsOf(page), w, h, spec);
    // A widget larger than a whole page would never fit; `size()` has already
    // clamped it, so this terminates.
    while (slot === null) {
      page += 1;
      slot = findFreeSlot(occupantsOf(page), w, h, spec);
    }
    const rect: CellRect = { ...slot, w, h };
    occupantsOf(page).push(rect);
    result.set(widget.id, { ...rect, page });
  }

  return result;
}

/** Highest page index in use, plus one. Always at least 1. */
export function pageCount(placements: Iterable<Placement>): number {
  let max = 0;
  for (const p of placements) {
    if (p.page > max) max = p.page;
  }
  return max + 1;
}

/**
 * Move a widget to `target`, pushing aside whatever it lands on.
 *
 * Displaced widgets are re-packed rather than swapped: swapping only has an
 * obvious meaning when both widgets are the same size, and silently resizing
 * someone else's widget to force a swap is worse than moving it.
 *
 * Returns a new map; the input is not mutated.
 */
export function moveWidget(
  placements: Map<string, Placement>,
  id: string,
  target: Placement,
  spec: GridSpec = DEFAULT_GRID,
): Map<string, Placement> {
  const moving = placements.get(id);
  if (!moving) return placements;

  const clamped = clampRect({ ...target, w: moving.w, h: moving.h }, spec);
  const next: Placement = { ...clamped, page: Math.max(0, target.page) };

  const result = new Map<string, Placement>();
  result.set(id, next);

  // Anything the move landed on gets re-packed after the survivors are fixed,
  // so a displaced widget cannot itself displace a third party.
  const displaced: { id: string; placement: Placement }[] = [];
  const occupied = new Map<number, CellRect[]>();
  occupied.set(next.page, [next]);

  for (const [otherId, placement] of placements) {
    if (otherId === id) continue;
    if (placement.page === next.page && rectsOverlap(placement, next)) {
      displaced.push({ id: otherId, placement });
      continue;
    }
    const list = occupied.get(placement.page) ?? [];
    list.push(placement);
    occupied.set(placement.page, list);
    result.set(otherId, placement);
  }

  for (const { id: displacedId, placement } of displaced) {
    let page = next.page;
    let slot = findFreeSlot(occupied.get(page) ?? [], placement.w, placement.h, spec);
    while (slot === null) {
      page += 1;
      slot = findFreeSlot(occupied.get(page) ?? [], placement.w, placement.h, spec);
    }
    const rect: Placement = { ...slot, w: placement.w, h: placement.h, page };
    const list = occupied.get(page) ?? [];
    list.push(rect);
    occupied.set(page, list);
    result.set(displacedId, rect);
  }

  return result;
}

/**
 * Resize a widget in place, refusing sizes that would overlap a neighbour.
 *
 * Returns the original map when the resize is not possible, so a drag that
 * grows past an obstacle simply stops there instead of shoving widgets around
 * mid-gesture.
 */
export function resizeWidget(
  placements: Map<string, Placement>,
  id: string,
  w: number,
  h: number,
  spec: GridSpec = DEFAULT_GRID,
): Map<string, Placement> {
  const current = placements.get(id);
  if (!current) return placements;

  const candidate = clampRect({ x: current.x, y: current.y, w, h }, spec);
  const others: CellRect[] = [];
  for (const [otherId, placement] of placements) {
    if (otherId !== id && placement.page === current.page) others.push(placement);
  }

  if (!fits(candidate, others, spec)) return placements;

  const result = new Map(placements);
  result.set(id, { ...candidate, page: current.page });
  return result;
}

/**
 * Drop empty pages and pull later pages forward.
 *
 * Without this, deleting the only widget on page 2 of 4 leaves a blank page
 * the user has to swipe through.
 */
export function compactPages(placements: Map<string, Placement>): Map<string, Placement> {
  const used = [...new Set([...placements.values()].map((p) => p.page))].sort((a, b) => a - b);
  const remap = new Map(used.map((page, index) => [page, index]));

  const result = new Map<string, Placement>();
  for (const [id, placement] of placements) {
    result.set(id, { ...placement, page: remap.get(placement.page) ?? 0 });
  }
  return result;
}
