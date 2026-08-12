import { h } from "preact";
import { Icons } from "./Icons";
import { t } from "../lib/i18n";

/**
 * Right-click / long-press menu for a widget.
 *
 * Also the keyboard-free route for moving a widget between pages: dragging
 * onto a page dot is the fast path, but it needs a pointer and a steady hand,
 * and it cannot reach a page that does not exist yet.
 */

export interface WidgetContextMenuProps {
  x: number;
  y: number;
  /** Total pages, used to build the move-to list. */
  pages: number;
  /** Page the target widget currently sits on. */
  currentPage: number;
  onEdit: () => void;
  onClone: () => void;
  onDelete: () => void;
  onMoveToPage: (page: number) => void;
}

export function WidgetContextMenu(props: WidgetContextMenuProps) {
  const { x, y, pages, currentPage, onEdit, onClone, onDelete, onMoveToPage } = props;

  // Offering "move to a new page" keeps the menu useful when every existing
  // page is full, which is exactly when a user reaches for it.
  const targets = [...Array(pages).keys()].filter((p) => p !== currentPage);

  return h(
    "div",
    { class: "ctx-menu", style: { left: `${x}px`, top: `${y}px` } },
    h("button", { class: "ctx-item", onClick: onEdit }, h(Icons.edit, null), t("widget.context.edit")),
    h("button", { class: "ctx-item", onClick: onClone }, h(Icons.copy, null), t("widget.context.clone")),

    h("div", { class: "ctx-separator" }),
    h("div", { class: "ctx-label" }, t("dashboard.moveToPage")),
    h(
      "div",
      { class: "ctx-pages" },
      targets.map((p) =>
        h(
          "button",
          {
            key: `move-${p}`,
            class: "ctx-page-btn",
            onClick: () => onMoveToPage(p),
            title: `${t("dashboard.page")} ${p + 1}`,
          },
          String(p + 1),
        ),
      ),
      h(
        "button",
        {
          class: "ctx-page-btn ctx-page-new",
          onClick: () => onMoveToPage(pages),
          title: t("dashboard.addPage"),
        },
        "+",
      ),
    ),

    h("div", { class: "ctx-separator" }),
    h(
      "button",
      { class: "ctx-item ctx-danger", onClick: onDelete },
      h(Icons.close, null),
      t("widget.context.delete"),
    ),
  );
}
