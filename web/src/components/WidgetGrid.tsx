import { h } from "preact";
import { useState, useEffect } from "preact/hooks";
import { WidgetType, DashboardLayout } from "../lib/types";
import { fetchDashboard, saveDashboard } from "../lib/api";
import { buildDefaultWidget } from "./widgetHelpers";
import { WidgetLibrary } from "./WidgetLibrary";
import { WidgetWizard } from "./WidgetWizard";
import { WidgetContent } from "./WidgetContent";

export function WidgetGrid() {
  const [layout, setLayout] = useState<DashboardLayout>({
    widgets: [],
    columns: 3,
  });
  const [loading, setLoading] = useState(true);
  const [showLibrary, setShowLibrary] = useState(false);
  const [wizardWidget, setWizardWidget] = useState<string | null>(null);

  useEffect(() => {
    fetchDashboard()
      .then((data) => {
        setLayout(data);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  function persist(next: DashboardLayout) {
    setLayout(next);
    saveDashboard(next);
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

  if (loading)
    return h("div", { class: "dashboard-loading" }, "Loading dashboard...");

  const editing = layout.widgets.find((w) => w.id === wizardWidget) || null;

  return h(
    "div",
    { class: "dashboard-root" },
    h(
      "div",
      { class: "dashboard-header" },
      h("h2", null, "Dashboard"),
      h(
        "button",
        { class: "add-widget-btn", onClick: () => setShowLibrary(true) },
        "+ Add Widget",
      ),
    ),
    layout.widgets.length === 0
      ? h(
          "div",
          { class: "dashboard-empty" },
          h("div", { class: "empty-icon" }, "+"),
          h("div", { class: "empty-text" }, "No widgets added yet"),
          h("div", { class: "empty-sub" }, 'Click "Add Widget" to get started'),
        )
      : h(
          "div",
          {
            class: "dashboard-grid",
            style: { gridTemplateColumns: `repeat(${layout.columns}, 1fr)` },
          },
          layout.widgets.map((widget) =>
            h(
              "div",
              {
                key: widget.id,
                class: `dashboard-widget variant-${widget.settings.variant || "compact"}`,
                style: {
                  gridColumn: `span ${widget.colSpan}`,
                  gridRow: `span ${widget.rowSpan}`,
                },
              },
              h(
                "div",
                { class: "widget-header" },
                h("span", { class: "widget-title" }, widget.title),
                h(
                  "div",
                  { class: "widget-controls" },
                  h(
                    "button",
                    {
                      class: "widget-control-btn edit",
                      onClick: () => setWizardWidget(widget.id),
                    },
                    "E",
                  ),
                  h(
                    "button",
                    {
                      class: "widget-control-btn remove",
                      onClick: () => handleRemoveWidget(widget.id),
                    },
                    "X",
                  ),
                ),
              ),
              h(
                "div",
                { class: "widget-content" },
                h(WidgetContent, { widget }),
              ),
            ),
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
  );
}
