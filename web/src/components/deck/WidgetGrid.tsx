import { h } from "preact";
import { useState, useEffect, useCallback, useMemo } from "preact/hooks";
import { WidgetType, DashboardLayout, WidgetConfig } from "../../lib/types";
import { fetchDashboard, saveDashboard } from "../../lib/api";
import { buildDefaultWidget, generateId } from "../widgetHelpers";
import { WidgetLibrary } from "../WidgetLibrary";
import { WidgetWizard } from "../wizard";
import { WidgetContent } from "../WidgetContent";
import { WidgetContextMenu } from "./WidgetContextMenu";
import { ArrangeToolbar } from "./ArrangeToolbar";
import { DeckGrid } from "./DeckGrid";
import "./WidgetGrid.css";
import { Icons } from "../../ui/icons/Icons";
import { t } from "../../lib/i18n";
import {
  DEFAULT_GRID,
  Placement,
  moveWidget,
  normalizeLayout,
  pageCount,
} from "../../lib/deckLayout";

/**
 * Dashboard state owner.
 *
 * Deliberately thin on rendering: the deck surface, the arrange toolbar and
 * the context menu are their own components, and this one holds the layout,
 * persists it, and mediates between them.
 *
 * The arrange flag is owned by `App` instead of living here: the FAB menu is
 * what toggles it, and a menu entry cannot reach into a child's state.
 */

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  widgetId: string;
}

export interface WidgetGridProps {
  arranging: boolean;
  onToggleArrange: () => void;
}

export function WidgetGrid({ arranging, onToggleArrange }: WidgetGridProps) {
  const [layout, setLayout] = useState<DashboardLayout>({
    widgets: [],
    columns: DEFAULT_GRID.columns,
    rows: DEFAULT_GRID.rows,
    aspect: DEFAULT_GRID.aspect,
  });
  const [loading, setLoading] = useState(true);
  const [showLibrary, setShowLibrary] = useState(false);
  const [wizardWidget, setWizardWidget] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false, x: 0, y: 0, widgetId: "",
  });
  const [page, setPage] = useState(0);

  const rows = layout.rows ?? DEFAULT_GRID.rows;

  useEffect(() => {
    fetchDashboard()
      .then((data) => {
        setLayout(data);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  useEffect(() => {
    function handleAddWidgetFromFAB() {
      setShowLibrary(true);
    }
    window.addEventListener("sd:add-widget", handleAddWidgetFromFAB);
    return () => window.removeEventListener("sd:add-widget", handleAddWidgetFromFAB);
  }, []);

  useEffect(() => {
    function handleClickOutside() {
      if (contextMenu.visible) setContextMenu((prev) => ({ ...prev, visible: false }));
    }
    if (contextMenu.visible) {
      document.addEventListener("click", handleClickOutside);
      document.addEventListener("contextmenu", handleClickOutside);
    }
    return () => {
      document.removeEventListener("click", handleClickOutside);
      document.removeEventListener("contextmenu", handleClickOutside);
    };
  }, [contextMenu.visible]);

  function persist(next: DashboardLayout) {
    setLayout(next);
    saveDashboard(next);
  }

  /**
   * Current placements, mirroring what `DeckGrid` derives.
   *
   * Needed here too so the context menu knows which page a widget is on and
   * how many pages exist, without the grid having to report it upward.
   */
  const placements = useMemo(
    () =>
      normalizeLayout(
        layout.widgets.map((w) => ({
          id: w.id, w: w.colSpan, h: w.rowSpan, x: w.x, y: w.y, page: w.page,
        })),
        { ...DEFAULT_GRID, columns: layout.columns, rows },
      ),
    [layout.widgets, layout.columns, rows],
  );

  /**
   * Write completed drag/resize results back onto the widgets.
   *
   * The deck hands back placements for *every* widget, not just the one that
   * moved, because displacing one can re-pack others.
   */
  const applyPlacements = useCallback(
    (next: Map<string, Placement>) => {
      persist({
        ...layout,
        widgets: layout.widgets.map((w) => {
          const p = next.get(w.id);
          return p ? { ...w, x: p.x, y: p.y, page: p.page, colSpan: p.w, rowSpan: p.h } : w;
        }),
      });
    },
    [layout],
  );

  function setGrid(patch: { columns?: number; rows?: number }) {
    // Positions are left alone: `normalizeLayout` re-packs only what no
    // longer fits, preserving arrangements that survive the resize.
    persist({ ...layout, ...patch });
  }

  function handleAddWidget(type: WidgetType) {
    const widget = buildDefaultWidget(type);
    // New widgets land on the page being viewed, not always page 1.
    persist({ ...layout, widgets: [...layout.widgets, { ...widget, page }] });
    setShowLibrary(false);
    setWizardWidget(widget.id);
  }

  function handleSaveWidget(
    id: string,
    updates: {
      title?: string;
      colSpan?: number;
      rowSpan?: number;
      settings?: Record<string, any>;
    },
  ) {
    persist({
      ...layout,
      widgets: layout.widgets.map((w) => (w.id === id ? { ...w, ...updates } : w)),
    });
    setWizardWidget(null);
  }

  function handleRemoveWidget(id: string) {
    persist({ ...layout, widgets: layout.widgets.filter((w) => w.id !== id) });
    setWizardWidget(null);
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }

  function handleCloneWidget(id: string) {
    const original = layout.widgets.find((w) => w.id === id);
    if (!original) return;
    const clone: WidgetConfig = {
      ...original,
      id: generateId(),
      title: original.title + " (copy)",
      settings: { ...original.settings },
      // Cleared so the packer finds it a free slot instead of stacking it
      // exactly on top of the original.
      x: undefined,
      y: undefined,
      page,
    };
    persist({ ...layout, widgets: [...layout.widgets, clone] });
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }

  function handleMoveToPage(id: string, target: number) {
    const current = placements.get(id);
    if (!current) return;
    applyPlacements(
      moveWidget(
        placements,
        id,
        { x: current.x, y: current.y, w: current.w, h: current.h, page: target },
        { ...DEFAULT_GRID, columns: layout.columns, rows },
      ),
    );
    setContextMenu((prev) => ({ ...prev, visible: false }));
    setPage(target);
  }

  function showContextMenu(e: Event, widgetId: string) {
    e.preventDefault();
    e.stopPropagation();
    const me = e as MouseEvent;
    const touch = (e as TouchEvent).changedTouches?.[0];
    const clientX = me.clientX ?? touch?.clientX ?? 0;
    const clientY = me.clientY ?? touch?.clientY ?? 0;
    const menuW = 200;
    const menuH = 260;
    setContextMenu({
      visible: true,
      x: Math.min(clientX, window.innerWidth - menuW - 8),
      y: Math.min(clientY, window.innerHeight - menuH - 8),
      widgetId,
    });
  }

  const handleWidgetContextMenu = useCallback((e: Event, widgetId: string) => {
    const target = e.target as HTMLElement;
    if (target.closest(".ctx-menu")) return;
    showContextMenu(e, widgetId);
  }, []);

  if (loading) return h("div", { class: "dashboard-loading" }, t("dashboard.loading"));

  const editing = layout.widgets.find((w) => w.id === wizardWidget) || null;
  const menuTargetPage = placements.get(contextMenu.widgetId)?.page ?? 0;

  return h(
    "div",
    { class: "dashboard-root" },
    // The header row is gone: its title was decoration, its add-widget button
    // duplicated the FAB menu, and its edit toggle lives there now too. Only
    // the arrangement controls remain, and only while arranging.
    arranging &&
      h(ArrangeToolbar, {
        columns: layout.columns,
        rows,
        onGridChange: setGrid,
        onDone: onToggleArrange,
      }),

    layout.widgets.length === 0
      ? h(
          "div",
          { class: "dashboard-empty" },
          h("div", { class: "empty-icon" }, h(Icons.plus, null)),
          h("div", { class: "empty-text" }, t("dashboard.empty")),
          h("div", { class: "empty-sub" }, t("dashboard.emptyHint")),
        )
      : h(DeckGrid, {
          widgets: layout.widgets,
          columns: layout.columns,
          rows,
          aspect: layout.aspect ?? DEFAULT_GRID.aspect,
          fit: "fill",
          editing: arranging,
          page,
          onPageChange: setPage,
          onPlacementsChange: applyPlacements,
          onWidgetContextMenu: handleWidgetContextMenu,
          renderWidget: (widget: WidgetConfig) => h(WidgetContent, { widget }),
        }),

    contextMenu.visible &&
      h(WidgetContextMenu, {
        x: contextMenu.x,
        y: contextMenu.y,
        pages: pageCount(placements.values()),
        currentPage: menuTargetPage,
        onEdit: () => {
          setWizardWidget(contextMenu.widgetId);
          setContextMenu((prev) => ({ ...prev, visible: false }));
        },
        onClone: () => handleCloneWidget(contextMenu.widgetId),
        onDelete: () => handleRemoveWidget(contextMenu.widgetId),
        onMoveToPage: (target) => handleMoveToPage(contextMenu.widgetId, target),
      }),

    showLibrary &&
      h(WidgetLibrary, { onAdd: handleAddWidget, onClose: () => setShowLibrary(false) }),

    editing &&
      h(WidgetWizard, {
        widget: editing,
        columns: layout.columns,
        rows,
        onSave: handleSaveWidget,
        onRemove: () => handleRemoveWidget(editing.id),
        onClose: () => setWizardWidget(null),
      }),

  );
}
