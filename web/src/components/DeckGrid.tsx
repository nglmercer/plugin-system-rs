import { h } from "preact";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "preact/hooks";
import { WidgetConfig } from "../lib/types";
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
} from "../lib/deckLayout";

/**
 * A Stream Deck-style widget surface.
 *
 * Every position is computed up front by `deckLayout` and applied as an
 * absolute pixel box, rather than left to grid auto-flow. That is what makes
 * the layout stable: a widget sits exactly where the math says, changing one
 * widget never reshuffles the rest, and drag/resize are pure arithmetic on a
 * geometry the component already holds.
 *
 * Editing is opt-in. Outside edit mode a widget is a plain interactive
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
      /** Live pixel position, for the dragged element only. */
      left: number;
      top: number;
    }
  | {
      kind: "resize";
      id: string;
      pointerId: number;
      /** Live pixel size of the widget being resized. */
      width: number;
      height: number;
    };

export interface DeckGridProps {
  widgets: WidgetConfig[];
  columns: number;
  rows: number;
  aspect: number;
  gap?: number;
  padding?: number;
  editing: boolean;
  /** Page shown right now, owned by the parent so toolbars can drive it. */
  page: number;
  onPageChange: (page: number) => void;
  /** Persist new placements. Called once per completed gesture, not per frame. */
  onPlacementsChange: (placements: Map<string, Placement>) => void;
  renderWidget: (widget: WidgetConfig) => h.JSX.Element;
  onWidgetContextMenu?: (e: Event, id: string) => void;
}

export function DeckGrid(props: DeckGridProps) {
  const {
    widgets, columns, rows, aspect, editing, page,
    onPageChange, onPlacementsChange, renderWidget, onWidgetContextMenu,
  } = props;

  const containerRef = useRef<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(0);
  const [gesture, setGesture] = useState<Gesture>({ kind: "none" });

  const spec: GridSpec = useMemo(
    () => ({
      columns,
      rows,
      aspect,
      gap: props.gap ?? 12,
      padding: props.padding ?? 12,
    }),
    [columns, rows, aspect, props.gap, props.padding],
  );

  // Measure the container rather than the window: the deck may sit inside a
  // sidebar layout, and `window.innerWidth` would overstate the space.
  useLayoutEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const measure = () => {
      setWidth(el.clientWidth);
      // Leave room for the pager beneath the page.
      const top = el.getBoundingClientRect().top;
      setViewportHeight(Math.max(240, window.innerHeight - top - 64));
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
    () => computeGeometry(width || 1, spec, viewportHeight),
    [width, spec, viewportHeight],
  );

  /**
   * Placements derived from the widgets themselves.
   *
   * Recomputed from props rather than held in state, so the parent stays the
   * single source of truth and a save that fails cannot leave the deck
   * showing a layout the server does not have.
   */
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

  // Deleting the last widget on the final page must not strand the viewer on
  // a page that no longer exists.
  useEffect(() => {
    if (page > pages - 1) onPageChange(Math.max(0, pages - 1));
  }, [pages, page, onPageChange]);

  const commit = useCallback(
    (next: Map<string, Placement>) => onPlacementsChange(compactPages(next)),
    [onPlacementsChange],
  );

  // ---- Gestures ----------------------------------------------------------

  const startMove = useCallback(
    (e: PointerEvent, id: string) => {
      if (!editing) return;
      const placement = placements.get(id);
      if (!placement) return;

      const target = e.currentTarget as HTMLElement;
      target.setPointerCapture(e.pointerId);
      e.preventDefault();

      const box = rectToPixels(placement, geom);
      const local = localPoint(e, containerRef.current);
      setGesture({
        kind: "move",
        id,
        pointerId: e.pointerId,
        grabX: local.x - box.left,
        grabY: local.y - box.top,
        left: box.left,
        top: box.top,
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
      const local = localPoint(e, containerRef.current);

      if (gesture.kind === "move") {
        setGesture({ ...gesture, left: local.x - gesture.grabX, top: local.y - gesture.grabY });
        return;
      }

      const placement = placements.get(gesture.id);
      if (!placement) return;
      const origin = rectToPixels(placement, geom);
      setGesture({
        ...gesture,
        width: Math.max(geom.cellWidth * 0.5, local.x - origin.left),
        height: Math.max(geom.cellHeight * 0.5, local.y - origin.top),
      });
    },
    [gesture, geom, placements],
  );

  const onPointerUp = useCallback(
    (e: PointerEvent) => {
      if (gesture.kind === "none" || e.pointerId !== gesture.pointerId) return;

      if (gesture.kind === "move") {
        const cell = pixelsToCell(gesture.left, gesture.top, geom);
        const current = placements.get(gesture.id);
        if (current) {
          commit(
            moveWidget(
              placements,
              gesture.id,
              { ...cell, w: current.w, h: current.h, page },
              spec,
            ),
          );
        }
      } else {
        const w = pixelsToSpan(gesture.width, geom.cellWidth, geom.gap);
        const hSpan = pixelsToSpan(gesture.height, geom.cellHeight, geom.gap);
        commit(resizeWidget(placements, gesture.id, w, hSpan, spec));
      }

      setGesture({ kind: "none" });
    },
    [gesture, geom, placements, page, spec, commit],
  );

  // ---- Render ------------------------------------------------------------

  const visible = widgets.filter((w) => placements.get(w.id)?.page === page);
  const dragging = gesture.kind !== "none" ? gesture.id : null;

  /** Where a dragged widget would land, shown as an outline. */
  const dropHint = useMemo(() => {
    if (gesture.kind !== "move") return null;
    const current = placements.get(gesture.id);
    if (!current) return null;
    const cell = pixelsToCell(gesture.left, gesture.top, geom);
    const clamped = {
      x: Math.min(Math.max(0, cell.x), spec.columns - current.w),
      y: Math.min(Math.max(0, cell.y), spec.rows - current.h),
      w: current.w,
      h: current.h,
    };
    return rectToPixels(clamped, geom);
  }, [gesture, placements, geom, spec]);

  return h(
    "div",
    { class: "deck-root" },
    h(
      "div",
      {
        ref: containerRef,
        class: `deck-page${editing ? " is-editing" : ""}`,
        style: { height: `${geom.pageHeight}px` },
        onPointerMove,
        onPointerUp,
        onPointerCancel: onPointerUp,
      },

      // Cell guides, so the grid is legible while arranging.
      editing &&
        h(
          "div",
          { class: "deck-guides", "aria-hidden": "true" },
          Array.from({ length: spec.columns * spec.rows }, (_, i) => {
            const box = rectToPixels(
              { x: i % spec.columns, y: Math.floor(i / spec.columns), w: 1, h: 1 },
              geom,
            );
            return h("div", {
              key: `guide-${i}`,
              class: "deck-guide",
              style: pxStyle(box),
            });
          }),
        ),

      dropHint && h("div", { class: "deck-drop-hint", style: pxStyle(dropHint) }),

      visible.map((widget) => {
        const placement = placements.get(widget.id)!;
        const box = rectToPixels(placement, geom);
        const isDragging = dragging === widget.id;

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
              `variant-${widget.settings.variant || "compact"}`,
              isDragging ? "is-dragging" : "",
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

    pages > 1 &&
      h(
        "div",
        { class: "deck-pager" },
        h(
          "button",
          {
            class: "deck-pager-arrow",
            disabled: page === 0,
            onClick: () => onPageChange(Math.max(0, page - 1)),
            "aria-label": "Previous page",
          },
          "‹",
        ),
        h(
          "div",
          { class: "deck-dots" },
          Array.from({ length: pages }, (_, i) =>
            h("button", {
              key: `dot-${i}`,
              class: `deck-dot${i === page ? " is-active" : ""}`,
              onClick: () => onPageChange(i),
              "aria-label": `Page ${i + 1}`,
              "aria-current": i === page ? "true" : undefined,
            }),
          ),
        ),
        h(
          "button",
          {
            class: "deck-pager-arrow",
            disabled: page >= pages - 1,
            onClick: () => onPageChange(Math.min(pages - 1, page + 1)),
            "aria-label": "Next page",
          },
          "›",
        ),
      ),
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
