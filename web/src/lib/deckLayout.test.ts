import { describe, it, expect } from "vitest";
import {
  DEFAULT_GRID,
  GridSpec,
  clampRect,
  compactPages,
  computeGeometry,
  fits,
  findFreeSlot,
  moveWidget,
  normalizeLayout,
  pageCount,
  pixelsToCell,
  pixelsToSpan,
  rectToPixels,
  rectsOverlap,
  resizeWidget,
  Placement,
} from "./deckLayout";

const spec: GridSpec = { columns: 4, rows: 3, gap: 10, padding: 20, aspect: 2 };

describe("computeGeometry", () => {
  it("divides the container into equal cells, accounting for gaps and padding", () => {
    // 400 wide - 40 padding - 30 gaps = 330 across 4 cells.
    const geom = computeGeometry(400, spec);
    expect(geom.cellWidth).toBeCloseTo(82.5);
    // aspect 2 means cells are twice as wide as tall.
    expect(geom.cellHeight).toBeCloseTo(41.25);
  });

  it("produces a page exactly as wide as the container when height is free", () => {
    const geom = computeGeometry(400, spec);
    expect(geom.pageWidth).toBeCloseTo(400);
    expect(geom.offsetX).toBeCloseTo(0);
  });

  /// The rounding trap this module exists to avoid: cells plus gutters plus
  /// padding must reconstitute the container exactly, at any column count.
  it("never drifts, for any column count", () => {
    for (let columns = 1; columns <= 12; columns++) {
      const geom = computeGeometry(1000, { ...spec, columns });
      const total =
        geom.cellWidth * columns + geom.gap * (columns - 1) + geom.padding * 2;
      expect(total).toBeCloseTo(1000, 6);
    }
  });

  it("shrinks cells to honour a height limit, preserving aspect ratio", () => {
    const unbounded = computeGeometry(400, spec);
    // 170 is tight enough to force a shrink but stays above the readability
    // floor, which is exercised separately below.
    const bounded = computeGeometry(400, spec, 170);

    expect(bounded.pageHeight).toBeLessThanOrEqual(170 + 0.001);
    expect(bounded.cellWidth).toBeLessThan(unbounded.cellWidth);
    // The whole point: squashing is not allowed.
    expect(bounded.cellWidth / bounded.cellHeight).toBeCloseTo(spec.aspect);
  });

  it("centres the grid when a height limit makes it narrower than the container", () => {
    const geom = computeGeometry(400, spec, 170);
    expect(geom.pageWidth).toBeLessThan(400);
    expect(geom.offsetX).toBeCloseTo((400 - geom.pageWidth) / 2);
  });

  it("keeps readable cells rather than obeying an impossible height", () => {
    // 10px of height for 3 rows would give unusably tiny keys.
    const geom = computeGeometry(400, spec, 10);
    expect(geom.cellWidth).toBeGreaterThan(50);
  });

  it("survives degenerate input", () => {
    const geom = computeGeometry(0, { ...spec, columns: 0, rows: 0, aspect: 0 });
    expect(geom.columns).toBe(1);
    expect(geom.rows).toBe(1);
    expect(Number.isFinite(geom.cellWidth)).toBe(true);
    expect(Number.isFinite(geom.cellHeight)).toBe(true);
  });
});

describe("rectToPixels", () => {
  const geom = computeGeometry(400, spec);

  it("places the first cell at the padding origin", () => {
    const px = rectToPixels({ x: 0, y: 0, w: 1, h: 1 }, geom);
    expect(px.left).toBeCloseTo(20);
    expect(px.top).toBeCloseTo(20);
    expect(px.width).toBeCloseTo(geom.cellWidth);
  });

  it("advances by one cell plus one gap per column", () => {
    const px = rectToPixels({ x: 2, y: 1, w: 1, h: 1 }, geom);
    expect(px.left).toBeCloseTo(20 + 2 * (geom.cellWidth + 10));
    expect(px.top).toBeCloseTo(20 + 1 * (geom.cellHeight + 10));
  });

  it("lets a spanned widget reclaim the gutters it covers", () => {
    const px = rectToPixels({ x: 0, y: 0, w: 3, h: 2 }, geom);
    expect(px.width).toBeCloseTo(3 * geom.cellWidth + 2 * 10);
    expect(px.height).toBeCloseTo(2 * geom.cellHeight + 1 * 10);
  });

  /// A full-width widget must line up with the right edge exactly, or the
  /// deck looks subtly broken at the margin.
  it("makes a full-width widget span the whole content area", () => {
    const px = rectToPixels({ x: 0, y: 0, w: spec.columns, h: 1 }, geom);
    expect(px.left + px.width).toBeCloseTo(geom.pageWidth - geom.padding);
  });
});

describe("pixelsToCell", () => {
  const geom = computeGeometry(400, spec);

  it("round-trips every cell origin", () => {
    for (let y = 0; y < spec.rows; y++) {
      for (let x = 0; x < spec.columns; x++) {
        const px = rectToPixels({ x, y, w: 1, h: 1 }, geom);
        expect(pixelsToCell(px.left, px.top, geom)).toEqual({ x, y });
      }
    }
  });

  it("snaps forward past the halfway point", () => {
    const origin = rectToPixels({ x: 1, y: 0, w: 1, h: 1 }, geom);
    const stride = geom.cellWidth + geom.gap;
    expect(pixelsToCell(origin.left + stride * 0.4, origin.top, geom).x).toBe(1);
    expect(pixelsToCell(origin.left + stride * 0.6, origin.top, geom).x).toBe(2);
  });
});

describe("pixelsToSpan", () => {
  it("maps a rendered width back to the span that produced it", () => {
    const geom = computeGeometry(400, spec);
    for (let w = 1; w <= spec.columns; w++) {
      const px = rectToPixels({ x: 0, y: 0, w, h: 1 }, geom);
      expect(pixelsToSpan(px.width, geom.cellWidth, geom.gap)).toBe(w);
    }
  });

  it("never returns less than one cell", () => {
    expect(pixelsToSpan(-500, 80, 10)).toBe(1);
    expect(pixelsToSpan(0, 80, 10)).toBe(1);
  });
});

describe("clampRect", () => {
  it("pulls an out-of-bounds rect back onto the page", () => {
    expect(clampRect({ x: 9, y: 9, w: 2, h: 2 }, spec)).toEqual({ x: 2, y: 1, w: 2, h: 2 });
    expect(clampRect({ x: -3, y: -3, w: 1, h: 1 }, spec)).toEqual({ x: 0, y: 0, w: 1, h: 1 });
  });

  it("caps a widget larger than the page at the page size", () => {
    expect(clampRect({ x: 0, y: 0, w: 99, h: 99 }, spec)).toEqual({
      x: 0, y: 0, w: spec.columns, h: spec.rows,
    });
  });
});

describe("rectsOverlap", () => {
  it("detects overlap but treats edge-sharing as separate", () => {
    const a = { x: 0, y: 0, w: 2, h: 2 };
    expect(rectsOverlap(a, { x: 1, y: 1, w: 2, h: 2 })).toBe(true);
    // Directly adjacent, not overlapping.
    expect(rectsOverlap(a, { x: 2, y: 0, w: 1, h: 1 })).toBe(false);
    expect(rectsOverlap(a, { x: 0, y: 2, w: 1, h: 1 })).toBe(false);
  });
});

describe("findFreeSlot", () => {
  it("scans left-to-right then top-to-bottom", () => {
    expect(findFreeSlot([{ x: 0, y: 0, w: 1, h: 1 }], 1, 1, spec)).toEqual({ x: 1, y: 0 });
  });

  it("wraps to the next row when the first is full", () => {
    const row = Array.from({ length: 4 }, (_, x) => ({ x, y: 0, w: 1, h: 1 }));
    expect(findFreeSlot(row, 1, 1, spec)).toEqual({ x: 0, y: 1 });
  });

  it("returns null when nothing fits, rather than overlapping", () => {
    const full: { x: number; y: number; w: number; h: number }[] = [];
    for (let y = 0; y < spec.rows; y++) {
      for (let x = 0; x < spec.columns; x++) full.push({ x, y, w: 1, h: 1 });
    }
    expect(findFreeSlot(full, 1, 1, spec)).toBeNull();
  });

  it("finds room for a wide widget only where the whole span is clear", () => {
    // Block the middle of row 0 so only row 1 can take a 4-wide widget.
    expect(findFreeSlot([{ x: 1, y: 0, w: 1, h: 1 }], 4, 1, spec)).toEqual({ x: 0, y: 1 });
  });
});

describe("normalizeLayout", () => {
  it("packs unpositioned widgets in order", () => {
    const placed = normalizeLayout(
      [{ id: "a" }, { id: "b" }, { id: "c" }],
      spec,
    );
    expect(placed.get("a")).toEqual({ x: 0, y: 0, w: 1, h: 1, page: 0 });
    expect(placed.get("b")).toEqual({ x: 1, y: 0, w: 1, h: 1, page: 0 });
    expect(placed.get("c")).toEqual({ x: 2, y: 0, w: 1, h: 1, page: 0 });
  });

  /// Migration from the old flow layout, which had spans but no coordinates.
  it("honours legacy spans while assigning fresh positions", () => {
    const placed = normalizeLayout([{ id: "wide", w: 3, h: 2 }, { id: "small" }], spec);
    expect(placed.get("wide")).toEqual({ x: 0, y: 0, w: 3, h: 2, page: 0 });
    expect(placed.get("small")).toEqual({ x: 3, y: 0, w: 1, h: 1, page: 0 });
  });

  it("leaves a user's explicit position untouched", () => {
    const placed = normalizeLayout([{ id: "pinned", x: 3, y: 2, page: 0 }, { id: "auto" }], spec);
    expect(placed.get("pinned")).toEqual({ x: 3, y: 2, w: 1, h: 1, page: 0 });
    expect(placed.get("auto")).toEqual({ x: 0, y: 0, w: 1, h: 1, page: 0 });
  });

  it("repacks a widget whose stored position collides", () => {
    const placed = normalizeLayout(
      [{ id: "first", x: 0, y: 0, page: 0 }, { id: "clash", x: 0, y: 0, page: 0 }],
      spec,
    );
    expect(placed.get("first")).toEqual({ x: 0, y: 0, w: 1, h: 1, page: 0 });
    expect(placed.get("clash")).toEqual({ x: 1, y: 0, w: 1, h: 1, page: 0 });
  });

  it("spills onto a new page when the current one fills", () => {
    const widgets = Array.from({ length: spec.columns * spec.rows + 1 }, (_, i) => ({
      id: `w${i}`,
    }));
    const placed = normalizeLayout(widgets, spec);
    expect(placed.get("w12")).toEqual({ x: 0, y: 0, w: 1, h: 1, page: 1 });
    expect(pageCount(placed.values())).toBe(2);
  });

  it("never overlaps two widgets", () => {
    const widgets = Array.from({ length: 30 }, (_, i) => ({
      id: `w${i}`,
      w: (i % 3) + 1,
      h: (i % 2) + 1,
    }));
    const placed = normalizeLayout(widgets, spec);

    const byPage = new Map<number, Placement[]>();
    for (const p of placed.values()) {
      const list = byPage.get(p.page) ?? [];
      list.push(p);
      byPage.set(p.page, list);
    }
    for (const list of byPage.values()) {
      for (let i = 0; i < list.length; i++) {
        for (let j = i + 1; j < list.length; j++) {
          expect(rectsOverlap(list[i], list[j])).toBe(false);
        }
      }
    }
    expect(placed.size).toBe(30);
  });

  /// A widget bigger than a page must still land somewhere, or normalization
  /// would loop forever looking for a slot that cannot exist.
  it("clamps an oversized widget instead of hanging", () => {
    const placed = normalizeLayout([{ id: "huge", w: 99, h: 99 }], spec);
    expect(placed.get("huge")).toEqual({
      x: 0, y: 0, w: spec.columns, h: spec.rows, page: 0,
    });
  });
});

describe("moveWidget", () => {
  it("moves into empty space without touching anyone", () => {
    const start = normalizeLayout([{ id: "a" }, { id: "b" }], spec);
    const moved = moveWidget(start, "a", { x: 3, y: 2, w: 1, h: 1, page: 0 }, spec);
    expect(moved.get("a")).toEqual({ x: 3, y: 2, w: 1, h: 1, page: 0 });
    expect(moved.get("b")).toEqual(start.get("b"));
  });

  it("displaces the occupant of the target cell", () => {
    const start = normalizeLayout([{ id: "a" }, { id: "b" }], spec);
    const moved = moveWidget(start, "a", { x: 1, y: 0, w: 1, h: 1, page: 0 }, spec);
    expect(moved.get("a")).toEqual({ x: 1, y: 0, w: 1, h: 1, page: 0 });
    expect(moved.get("b")).not.toEqual({ x: 1, y: 0, w: 1, h: 1, page: 0 });
    expect(rectsOverlap(moved.get("a")!, moved.get("b")!)).toBe(false);
  });

  it("moves a widget across pages", () => {
    const start = normalizeLayout([{ id: "a" }, { id: "b" }], spec);
    const moved = moveWidget(start, "a", { x: 0, y: 0, w: 1, h: 1, page: 2 }, spec);
    expect(moved.get("a")!.page).toBe(2);
  });

  it("clamps a drop that lands off the page", () => {
    const start = normalizeLayout([{ id: "a", w: 2, h: 2 }], spec);
    const moved = moveWidget(start, "a", { x: 99, y: 99, w: 2, h: 2, page: 0 }, spec);
    expect(moved.get("a")).toEqual({ x: 2, y: 1, w: 2, h: 2, page: 0 });
  });

  it("is a no-op for an unknown id", () => {
    const start = normalizeLayout([{ id: "a" }], spec);
    expect(moveWidget(start, "ghost", { x: 0, y: 0, w: 1, h: 1, page: 0 }, spec)).toBe(start);
  });

  it("does not mutate the input", () => {
    const start = normalizeLayout([{ id: "a" }, { id: "b" }], spec);
    const before = start.get("a");
    moveWidget(start, "a", { x: 3, y: 2, w: 1, h: 1, page: 0 }, spec);
    expect(start.get("a")).toEqual(before);
  });
});

describe("resizeWidget", () => {
  it("grows into free space", () => {
    const start = normalizeLayout([{ id: "a" }], spec);
    const resized = resizeWidget(start, "a", 2, 2, spec);
    expect(resized.get("a")).toEqual({ x: 0, y: 0, w: 2, h: 2, page: 0 });
  });

  /// Growing over a neighbour stops at the obstacle rather than shoving it,
  /// so a resize drag stays predictable.
  it("refuses a size that would overlap a neighbour", () => {
    const start = normalizeLayout([{ id: "a" }, { id: "b" }], spec);
    expect(resizeWidget(start, "a", 2, 1, spec)).toBe(start);
  });

  it("caps growth at the page edge", () => {
    const start = normalizeLayout([{ id: "a" }], spec);
    const resized = resizeWidget(start, "a", 99, 99, spec);
    expect(resized.get("a")).toEqual({
      x: 0, y: 0, w: spec.columns, h: spec.rows, page: 0,
    });
  });

  it("always keeps at least one cell", () => {
    const start = normalizeLayout([{ id: "a", w: 2, h: 2 }], spec);
    const resized = resizeWidget(start, "a", 0, -5, spec);
    expect(resized.get("a")).toEqual({ x: 0, y: 0, w: 1, h: 1, page: 0 });
  });
});

describe("compactPages", () => {
  it("pulls later pages forward over an emptied one", () => {
    const placements = new Map<string, Placement>([
      ["a", { x: 0, y: 0, w: 1, h: 1, page: 0 }],
      ["b", { x: 0, y: 0, w: 1, h: 1, page: 2 }],
      ["c", { x: 1, y: 0, w: 1, h: 1, page: 5 }],
    ]);
    const compacted = compactPages(placements);
    expect(compacted.get("a")!.page).toBe(0);
    expect(compacted.get("b")!.page).toBe(1);
    expect(compacted.get("c")!.page).toBe(2);
  });

  it("preserves cell positions while renumbering pages", () => {
    const placements = new Map<string, Placement>([
      ["a", { x: 2, y: 1, w: 2, h: 1, page: 7 }],
    ]);
    expect(compactPages(placements).get("a")).toEqual({ x: 2, y: 1, w: 2, h: 1, page: 0 });
  });
});

describe("pageCount", () => {
  it("is at least one, even with no widgets", () => {
    expect(pageCount([])).toBe(1);
  });
});

describe("DEFAULT_GRID", () => {
  it("is self-consistent", () => {
    expect(DEFAULT_GRID.columns).toBeGreaterThan(0);
    expect(DEFAULT_GRID.rows).toBeGreaterThan(0);
    expect(DEFAULT_GRID.aspect).toBeGreaterThan(0);
  });
});
