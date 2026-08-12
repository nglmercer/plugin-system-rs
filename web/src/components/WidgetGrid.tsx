import { h } from "preact";
import { useState, useEffect, useRef, useCallback } from "preact/hooks";
import { WidgetType, DashboardLayout, WidgetConfig } from "../lib/types";
import { DeckGrid } from "./DeckGrid";
import { DEFAULT_GRID, Placement } from "../lib/deckLayout";
import { fetchDashboard, saveDashboard } from "../lib/api";
import { buildDefaultWidget, generateId } from "./widgetHelpers";
import { WidgetLibrary } from "./WidgetLibrary";
import { WidgetWizard } from "./WidgetWizard";
import { WidgetContent } from "./WidgetContent";
import { Icons } from "./Icons";
import { t } from "../lib/i18n";
import { CssEditor } from "./CssEditor";

/** Small +/- control used by the arrange toolbar. */
function stepper(
  value: number,
  onChange: (next: number) => void,
  min: number,
  max: number,
) {
  return h(
    "div",
    { class: "deck-stepper" },
    h(
      "button",
      {
        onClick: () => onChange(value - 1),
        disabled: value <= min,
        "aria-label": "Decrease",
      },
      "-",
    ),
    h("span", null, String(value)),
    h(
      "button",
      {
        onClick: () => onChange(value + 1),
        disabled: value >= max,
        "aria-label": "Increase",
      },
      "+",
    ),
  );
}

interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  widgetId: string;
}

export function WidgetGrid() {
  const [layout, setLayout] = useState<DashboardLayout>({
    widgets: [],
    columns: DEFAULT_GRID.columns,
    rows: DEFAULT_GRID.rows,
    aspect: DEFAULT_GRID.aspect,
  });
  const [arranging, setArranging] = useState(false);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [showLibrary, setShowLibrary] = useState(false);
  const [wizardWidget, setWizardWidget] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false, x: 0, y: 0, widgetId: "",
  });
  const [showCssEditor, setShowCssEditor] = useState(false);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

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
      if (contextMenu.visible) {
        setContextMenu((prev) => ({ ...prev, visible: false }));
      }
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
   * Write completed drag/resize results back onto the widgets.
   *
   * The deck hands back placements for *every* widget, not just the one that
   * moved, because displacing one can re-pack others. Applying the whole map
   * keeps the saved layout and the rendered one identical.
   */
  function handlePlacementsChange(placements: Map<string, Placement>) {
    persist({
      ...layout,
      widgets: layout.widgets.map((w) => {
        const p = placements.get(w.id);
        return p
          ? { ...w, x: p.x, y: p.y, page: p.page, colSpan: p.w, rowSpan: p.h }
          : w;
      }),
    });
  }

  /**
   * Change the page shape.
   *
   * Positions are deliberately left alone: `normalizeLayout` re-packs anything
   * that no longer fits, on the next render, and keeps everything that still
   * does. Rewriting coordinates here would discard arrangements that survive
   * the resize perfectly well.
   */
  function setGrid(patch: { columns?: number; rows?: number }) {
    persist({ ...layout, ...patch });
  }

  function handleAddWidget(type: WidgetType) {
    const widget = buildDefaultWidget(type);
    persist({ ...layout, widgets: [...layout.widgets, widget] });
    setShowLibrary(false);
    setWizardWidget(widget.id);
  }

  function handleSaveWidget(
    id: string,
    updates: {
      title?: string;
      colSpan?: number;
      settings?: Record<string, any>;
    },
  ) {
    persist({
      ...layout,
      widgets: layout.widgets.map((w) =>
        w.id === id ? { ...w, ...updates } : w,
      ),
    });
    setWizardWidget(null);
  }

  function handleRemoveWidget(id: string) {
    persist({ ...layout, widgets: layout.widgets.filter((w) => w.id !== id) });
    setWizardWidget(null);
  }

  function handleCloneWidget(id: string) {
    const original = layout.widgets.find((w) => w.id === id);
    if (!original) return;
    const clone: WidgetConfig = {
      ...original,
      id: generateId(),
      title: original.title + " (copy)",
      settings: { ...original.settings },
    };
    persist({ ...layout, widgets: [...layout.widgets, clone] });
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }

  function handleCssChange(css: string) {
    persist({ ...layout, customCss: css });
  }

  function showContextMenu(e: Event, widgetId: string) {
    e.preventDefault();
    e.stopPropagation();
    const me = e as MouseEvent;
    const touch = (e as TouchEvent).changedTouches?.[0];
    const clientX = me.clientX ?? touch?.clientX ?? 0;
    const clientY = me.clientY ?? touch?.clientY ?? 0;
    const menuW = 180;
    const menuH = 140;
    const x = Math.min(clientX, window.innerWidth - menuW - 8);
    const y = Math.min(clientY, window.innerHeight - menuH - 8);
    setContextMenu({ visible: true, x, y, widgetId });
  }

  const handlePointerDown = useCallback((e: Event, widgetId: string) => {
    const target = e.target as HTMLElement;
    if (target.closest(".ctx-menu") || target.closest(".ctx-item")) return;

    if (e.type === "contextmenu") {
      showContextMenu(e, widgetId);
      return;
    }

    longPressTimer.current = setTimeout(() => {
      showContextMenu(e, widgetId);
    }, 500);
  }, []);

  const handlePointerUp = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }, []);

  const handlePointerMove = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }, []);

  useEffect(() => {
    return () => {
      if (longPressTimer.current) clearTimeout(longPressTimer.current);
    };
  }, []);

  useEffect(() => {
    let styleEl = document.getElementById("custom-dashboard-css");
    if (!styleEl) {
      styleEl = document.createElement("style");
      styleEl.id = "custom-dashboard-css";
      document.head.appendChild(styleEl);
    }
    styleEl.textContent = layout.customCss || "";
    return () => {
      if (styleEl && styleEl.parentNode) {
        styleEl.parentNode.removeChild(styleEl);
      }
    };
  }, [layout.customCss]);

  if (loading)
    return h("div", { class: "dashboard-loading" }, t("dashboard.loading"));

  const editing = layout.widgets.find((w) => w.id === wizardWidget) || null;

  return h(
    "div",
    { class: "dashboard-root" },
    h(
      "div",
      { class: "dashboard-header" },
      h("h2", null, t("dashboard.title")),
      h(
        "div",
        { class: "dashboard-header-actions deck-toolbar" },
        arranging &&
          h(
            "div",
            { class: "deck-toolbar-group" },
            h("label", null, t("dashboard.columns")),
            stepper(
              layout.columns,
              (v) => setGrid({ columns: v }),
              1,
              12,
            ),
          ),
        arranging &&
          h(
            "div",
            { class: "deck-toolbar-group" },
            h("label", null, t("dashboard.rows")),
            stepper(
              layout.rows ?? DEFAULT_GRID.rows,
              (v) => setGrid({ rows: v }),
              1,
              12,
            ),
          ),
        h(
          "button",
          {
            class: `deck-edit-toggle${arranging ? " is-active" : ""}`,
            onClick: () => setArranging((v) => !v),
          },
          arranging ? t("dashboard.done") : t("dashboard.edit"),
        ),
        h(
          "button",
          { class: "add-widget-btn", onClick: () => setShowCssEditor(true) },
          "{ }",
        ),
        h(
          "button",
          { class: "add-widget-btn", onClick: () => setShowLibrary(true) },
          h(Icons.plus, null),
          t("dashboard.addWidget"),
        ),
      ),
    ),
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
          rows: layout.rows ?? DEFAULT_GRID.rows,
          aspect: layout.aspect ?? DEFAULT_GRID.aspect,
          editing: arranging,
          page,
          onPageChange: setPage,
          onPlacementsChange: handlePlacementsChange,
          onWidgetContextMenu: (e: Event, id: string) => handlePointerDown(e, id),
          renderWidget: (widget: WidgetConfig) => h(WidgetContent, { widget }),
        }),
    contextMenu.visible &&
      h(
        "div",
        {
          class: "ctx-menu",
          style: { left: contextMenu.x + "px", top: contextMenu.y + "px" },
        },
        h(
          "button",
          {
            class: "ctx-item",
            onClick: () => {
              setWizardWidget(contextMenu.widgetId);
              setContextMenu((prev) => ({ ...prev, visible: false }));
            },
          },
          h(Icons.edit, null),
          t("widget.context.edit"),
        ),
        h(
          "button",
          {
            class: "ctx-item",
            onClick: () => {
              handleCloneWidget(contextMenu.widgetId);
            },
          },
          h(Icons.copy, null),
          t("widget.context.clone"),
        ),
        h("div", { class: "ctx-separator" }),
        h(
          "button",
          {
            class: "ctx-item ctx-danger",
            onClick: () => {
              handleRemoveWidget(contextMenu.widgetId);
              setContextMenu((prev) => ({ ...prev, visible: false }));
            },
          },
          h(Icons.close, null),
          t("widget.context.delete"),
        ),
      ),
    showLibrary &&
      h(WidgetLibrary, {
        onAdd: handleAddWidget,
        onClose: () => setShowLibrary(false),
      }),
    editing &&
      h(WidgetWizard, {
        widget: editing,
        columns: layout.columns,
        onSave: (id, updates) => handleSaveWidget(id, updates),
        onRemove: () => handleRemoveWidget(editing.id),
        onClose: () => setWizardWidget(null),
      }),
    showCssEditor &&
      h(CssEditor, {
        value: layout.customCss || "",
        onChange: handleCssChange,
        onClose: () => setShowCssEditor(false),
      }),
  );
}
