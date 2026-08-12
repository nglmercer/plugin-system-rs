import { h } from "preact";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "preact/hooks";
import { WidgetConfig } from "../../lib/types";
import {
  GridSpec,
  Geometry,
  Placement,
  compactPages,
  computeGeometry,
  moveWidget,
  normalizeLayout,
  pageCount,
  pixelsToCell,
  pixelsToSpan,
  rectToPixels,
  resizeWidget,
} from "../../lib/deckLayout";
import { DeckPager } from "./DeckPager";
import "./deck.css";

/**
 * A Stream Deck-style widget surface.
 *
 * Every position is computed up front by `deckLayout` and applied as an
 * absolute pixel box, rather than left to grid auto-flow. That is what makes
 * the layout stable: a widget sits exactly where the math says, changing one
 * widget never reshuffles the rest, and drag/resize are pure arithmetic on a
 * geometry the component already holds.
 *
 * Editing is opt-in. Outside arrange mode a widget is a plain interactive
 * control, so a slider drag is never mistaken for a layout drag.
 */

/** A gesture in progress. */
type Gesture =
  | { kind: "none" }
  | {
      kind: "move";
      id: string;
      pointerId: number;
      /** Offset from the widget's top-left to the pointer, in px. */
      grabX: number;
      grabY: number;
      /** Live pixel position, clamped to the page. */
      left: number;
      top: number;
      /** Page under the pointer when it is over the pager, else null. */
      dropPage: number | null;
    }
  | {
      kind: "resize";
      id: string;
      pointerId: number;
      width: number;
      height: number;
    };

export interface DeckGridProps {
  widgets: WidgetConfig[];
  columns: number;
  rows: number;
  aspect: number;
  /** `"fill"` covers the container; `"aspect"` keeps cell proportions. */
  fit?: "fill" | "aspect";
  gap?: number;
  padding?: number;
  editing: boolean;
  page: number;
  onPageChange: (page: number) => void;
  /** Persist new placements. Called once per completed gesture, not per frame. */
  onPlacementsChange: (placements: Map<string, Placement>) => void;
  renderWidget: (widget: WidgetConfig) => h.JSX.Element;
  onWidgetContextMenu?: (e: Event, id: string) => void;
}

export function DeckGrid(props: DeckGridProps) {
  const {
    widgets, columns, rows, aspect, fit, editing, page,
    onPageChange, onPlacementsChange, renderWidget, onWidgetContextMenu,
  } = props;

  const rootRef = useRef<HTMLDivElement | null>(null);
  const pageRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [gesture, setGesture] = useState<Gesture>({ kind: "none" });

  const spec: GridSpec = useMemo(
    () => ({
      columns,
      rows,
      aspect,
      fit: fit ?? "fill",
      gap: props.gap ?? 12,
      padding: props.padding ?? 12,
    }),
    [columns, rows, aspect, fit, props.gap, props.padding],
  );

  /**
   * Measure the element the page actually occupies.
   *
   * Its height comes from CSS (`flex: 1` inside a viewport-height column), so
   * reading it back gives the deck exactly the space the layout granted —
   * rather than guessing from `window.innerHeight` minus assorted chrome, which
   * is what left a dead band at the bottom before.
   */
  useLayoutEffect(() => {
    const el = pageRef.current;
    if (!el) return;

    const measure = () => {
      const rect = el.getBoundingClientRect();
      setSize({ width: el.clientWidth, height: rect.height });
    };

    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    window.addEventListener("resize", measure);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);

  const geom: Geometry = useMemo(
    () => computeGeometry(size.width || 1, spec, size.height || undefined),
    [size, spec],
  );

  const placements = useMemo(
    () =>
      normalizeLayout(
        widgets.map((w) => ({
          id: w.id,
          w: w.colSpan,
          h: w.rowSpan,
          x: w.x,
          y: w.y,
          page: w.page,
        })),
        spec,
      ),
    [widgets, spec],
  );

  const pages = pageCount(placements.values());

  useEffect(() => {
    if (page > pages - 1) onPageChange(Math.max(0, pages - 1));
  }, [pages, page, onPageChange]);

  const commit = useCallback(
    (next: Map<string, Placement>) => onPlacementsChange(compactPages(next)),
    [onPlacementsChange],
  );

  // ---- Gestures ----------------------------------------------------------

  /** Keep a dragged box inside the page, so it can never cause overflow. */
  const clampToPage = useCallback(
    (left: number, top: number, box: { width: number; height: number }) => ({
      left: Math.min(Math.max(geom.offsetX, left), geom.offsetX + geom.pageWidth - box.width),
      top: Math.min(Math.max(0, top), Math.max(0, geom.pageHeight - box.height)),
    }),
    [geom],
  );

  /** Which page dot, if any, the pointer is over. */
  function pageUnderPointer(e: PointerEvent): number | null {
    const el = document.elementFromPoint(e.clientX, e.clientY);
    const dot = el?.closest?.("[data-deck-page]") as HTMLElement | null;
    if (!dot) return null;
    const value = Number(dot.dataset.deckPage);
    return Number.isFinite(value) ? value : null;
  }

  const startMove = useCallback(
    (e: PointerEvent, id: string) => {
      if (!editing) return;
      const placement = placements.get(id);
      if (!placement) return;

      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
      e.preventDefault();

      const box = rectToPixels(placement, geom);
      const local = localPoint(e, pageRef.current);
      setGesture({
        kind: "move",
        id,
        pointerId: e.pointerId,
        grabX: local.x - box.left,
        grabY: local.y - box.top,
        left: box.left,
        top: box.top,
        dropPage: null,
      });
    },
    [editing, placements, geom],
  );

  const startResize = useCallback(
    (e: PointerEvent, id: string) => {
      if (!editing) return;
      const placement = placements.get(id);
      if (!placement) return;

      e.stopPropagation();
      e.preventDefault();
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);

      const box = rectToPixels(placement, geom);
      setGesture({
        kind: "resize",
        id,
        pointerId: e.pointerId,
        width: box.width,
        height: box.height,
      });
    },
    [editing, placements, geom],
  );

  const onPointerMove = useCallback(
    (e: PointerEvent) => {
      if (gesture.kind === "none" || e.pointerId !== gesture.pointerId) return;
      const local = localPoint(e, pageRef.current);

      if (gesture.kind === "move") {
        const placement = placements.get(gesture.id);
        if (!placement) return;
        const box = rectToPixels(placement, geom);
        const clamped = clampToPage(local.x - gesture.grabX, local.y - gesture.grabY, box);
        setGesture({ ...gesture, ...clamped, dropPage: pageUnderPointer(e) });
        return;
      }

      const placement = placements.get(gesture.id);
      if (!placement) return;
      const origin = rectToPixels(placement, geom);
      setGesture({
        ...gesture,
        // Bounded by the page edge so a resize cannot overflow either.
        width: Math.min(
          Math.max(geom.cellWidth * 0.5, local.x - origin.left),
          geom.offsetX + geom.pageWidth - origin.left - geom.padding,
        ),
        height: Math.min(
          Math.max(geom.cellHeight * 0.5, local.y - origin.top),
          Math.max(geom.cellHeight, geom.pageHeight - origin.top - geom.padding),
        ),
      });
    },
    [gesture, geom, placements, clampToPage],
  );

  const onPointerUp = useCallback(
    (e: PointerEvent) => {
      if (gesture.kind === "none" || e.pointerId !== gesture.pointerId) return;

      if (gesture.kind === "move") {
        const current = placements.get(gesture.id);
        if (current) {
          // Released over a page dot: move to that page, keeping the cell
          // position where it will fit. Otherwise place within this page.
          const target = gesture.dropPage;
          if (target !== null && target !== current.page) {
            commit(
              moveWidget(
                placements,
                gesture.id,
                { x: current.x, y: current.y, w: current.w, h: current.h, page: target },
                spec,
              ),
            );
            onPageChange(target);
          } else {
            const cell = pixelsToCell(gesture.left, gesture.top, geom);
            commit(
              moveWidget(
                placements,
                gesture.id,
                { ...cell, w: current.w, h: current.h, page },
                spec,
              ),
            );
          }
        }
      } else {
        const w = pixelsToSpan(gesture.width, geom.cellWidth, geom.gap);
        const hSpan = pixelsToSpan(gesture.height, geom.cellHeight, geom.gap);
        commit(resizeWidget(placements, gesture.id, w, hSpan, spec));
      }

      setGesture({ kind: "none" });
    },
    [gesture, geom, placements, page, spec, commit, onPageChange],
  );

  // ---- Render ------------------------------------------------------------

  const visible = widgets.filter((w) => placements.get(w.id)?.page === page);
  const draggingId = gesture.kind !== "none" ? gesture.id : null;

  /** Where a dragged widget would land, shown as an outline. */
  const dropHint = useMemo(() => {
    if (gesture.kind !== "move" || gesture.dropPage !== null) return null;
    const current = placements.get(gesture.id);
    if (!current) return null;
    const cell = pixelsToCell(gesture.left, gesture.top, geom);
    return rectToPixels(
      {
        x: Math.min(Math.max(0, cell.x), spec.columns - current.w),
        y: Math.min(Math.max(0, cell.y), spec.rows - current.h),
        w: current.w,
        h: current.h,
      },
      geom,
    );
  }, [gesture, placements, geom, spec]);

  return h(
    "div",
    { class: "deck-root", ref: rootRef },
    h(
      "div",
      {
        ref: pageRef,
        class: `deck-page${editing ? " is-editing" : ""}`,
        onPointerMove,
        onPointerUp,
        onPointerCancel: onPointerUp,
      },

      editing &&
        h(
          "div",
          { class: "deck-guides", "aria-hidden": "true" },
          Array.from({ length: spec.columns * spec.rows }, (_, i) => {
            const box = rectToPixels(
              { x: i % spec.columns, y: Math.floor(i / spec.columns), w: 1, h: 1 },
              geom,
            );
            return h("div", { key: `guide-${i}`, class: "deck-guide", style: pxStyle(box) });
          }),
        ),

      dropHint && h("div", { class: "deck-drop-hint", style: pxStyle(dropHint) }),

      visible.map((widget) => {
        const placement = placements.get(widget.id)!;
        const box = rectToPixels(placement, geom);
        const isDragging = draggingId === widget.id;

        const style =
          isDragging && gesture.kind === "move"
            ? pxStyle({ ...box, left: gesture.left, top: gesture.top })
            : isDragging && gesture.kind === "resize"
              ? pxStyle({ ...box, width: gesture.width, height: gesture.height })
              : pxStyle(box);

        return h(
          "div",
          {
            key: widget.id,
            class: [
              "deck-widget",
              isDragging ? "is-dragging" : "",
              // Faded while heading to another page, so it reads as leaving.
              isDragging && gesture.kind === "move" && gesture.dropPage !== null
                ? "is-leaving"
                : "",
            ]
              .filter(Boolean)
              .join(" "),
            style,
            onPointerDown: editing ? (e: PointerEvent) => startMove(e, widget.id) : undefined,
            onContextMenu: onWidgetContextMenu
              ? (e: Event) => onWidgetContextMenu(e, widget.id)
              : undefined,
          },
          h("div", { class: "deck-widget-content" }, renderWidget(widget)),
          editing &&
            h("div", {
              class: "deck-resize-handle",
              title: "Resize",
              onPointerDown: (e: PointerEvent) => startResize(e, widget.id),
            }),
        );
      }),
    ),

    h(DeckPager, {
      pages,
      page,
      onPageChange,
      dragging: gesture.kind === "move",
      dropPage: gesture.kind === "move" ? gesture.dropPage : null,
      // Only offered while arranging: a spare empty page is clutter otherwise.
      onAddPage: editing ? () => onPageChange(pages) : undefined,
    }),
  );
}

/** Pointer position relative to the page element. */
function localPoint(e: PointerEvent, container: HTMLElement | null) {
  if (!container) return { x: e.clientX, y: e.clientY };
  const rect = container.getBoundingClientRect();
  return { x: e.clientX - rect.left, y: e.clientY - rect.top };
}

function pxStyle(box: { left: number; top: number; width: number; height: number }) {
  return {
    left: `${box.left}px`,
    top: `${box.top}px`,
    width: `${box.width}px`,
    height: `${box.height}px`,
  };
}
