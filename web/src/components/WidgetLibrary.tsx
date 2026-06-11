import { h } from "preact";
import { WidgetType } from "../lib/types";
import { WIDGET_CATALOG } from "./widgetHelpers";

export function WidgetLibrary({ onAdd, onClose }: { onAdd: (t: WidgetType) => void; onClose: () => void }) {
  return h("div", { class: "widget-library-overlay", onClick: onClose },
    h("div", { class: "widget-library-modal", onClick: (e: Event) => e.stopPropagation() },
      h("div", { class: "library-header" },
        h("span", null, "Widget Library"),
        h("button", { class: "picker-close", onClick: onClose }, "X"),
      ),
      h("div", { class: "library-grid" },
        WIDGET_CATALOG.map((item) =>
          h("button", { class: "library-item", key: item.type, onClick: () => onAdd(item.type) },
            h("div", { class: "library-icon" }, item.icon),
            h("div", { class: "library-label" }, item.label),
            h("div", { class: "library-desc" }, item.description),
          )
        ),
      ),
    ),
  );
}
