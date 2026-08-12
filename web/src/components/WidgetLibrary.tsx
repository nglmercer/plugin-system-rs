import { h } from "preact";
import { useMemo, useState } from "preact/hooks";
import { t, tOr } from "../lib/i18n";
import {
  checkRequirements,
  summarizeUnmet,
  useHostStatus,
  widgetsByCategory,
  WidgetDefinition,
} from "../widgets";
import { Modal, ModalClose } from "./Modal";
import "./Modal.css";
import "./WidgetLibrary.css";

/**
 * The widget picker.
 *
 * Three things changed from the flat grid it replaced: widgets are grouped by
 * category so a list that grows with every plugin stays navigable, there is a
 * filter for when it grows anyway, and a widget whose plugin is missing is
 * shown as unavailable with the reason rather than looking identical to one
 * that will work.
 *
 * Unavailable widgets are still addable. Someone laying out a dashboard before
 * starting OBS is doing something reasonable, and a picker that hides options
 * based on transient state is a picker people stop trusting.
 */
export function WidgetLibrary({
  onAdd,
  onClose,
}: {
  onAdd: (type: string) => void;
  onClose: () => void;
}) {
  const host = useHostStatus();
  const [query, setQuery] = useState("");

  const groups = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return widgetsByCategory()
      .map((group) => ({
        ...group,
        widgets: group.widgets.filter((widget) => matches(widget, needle)),
      }))
      .filter((group) => group.widgets.length > 0);
  }, [query, host]);

  return h(
    Modal,
    { onClose, sheet: true, class: "widget-library-modal" },
    h(
      "div",
      { class: "library-header" },
      h("span", { class: "library-title" }, t("widget.library")),
      h(ModalClose, { onClose, label: t("common.close") }),
    ),
      h("input", {
        class: "library-search",
        type: "search",
        value: query,
        placeholder: t("widget.searchPlaceholder"),
        onInput: (e: Event) => setQuery((e.target as HTMLInputElement).value),
      }),
      h(
        "div",
        { class: "library-scroll" },
        groups.length === 0
          ? h("div", { class: "library-empty" }, t("widget.noMatches"))
          : groups.map((group) =>
              h(
                "section",
                { class: "library-section", key: group.category },
                h(
                  "h4",
                  { class: "library-section-title" },
                  t(`widget.categories.${group.category}`),
                ),
                h(
                  "div",
                  { class: "library-grid" },
                  group.widgets.map((widget) =>
                    h(LibraryTile, { key: widget.type, widget, onAdd }),
                  ),
                ),
              ),
            ),
    ),
  );
}

function LibraryTile({
  widget,
  onAdd,
}: {
  widget: WidgetDefinition;
  onAdd: (type: string) => void;
}) {
  const host = useHostStatus();
  const requirements = checkRequirements(widget, host);
  const unavailable = !requirements.satisfied;

  return h(
    "button",
    {
      class: `library-item ${unavailable ? "unavailable" : ""}`,
      type: "button",
      title: unavailable ? requirements.unmet[0]?.remedy : widget.description,
      onClick: () => onAdd(widget.type),
    },
    h("div", { class: "library-icon" }, h(widget.icon, null)),
    h(
      "div",
      { class: "library-label" },
      tOr(`widget.types.${widget.type}`, widget.label),
    ),
    h("div", { class: "library-desc" }, widget.description),
    widget.source === "external"
      ? h("span", { class: "library-badge external" }, t("widget.fromPlugin"))
      : null,
    unavailable
      ? h("span", { class: "library-badge blocked" }, summarizeUnmet(requirements))
      : null,
  );
}

function matches(widget: WidgetDefinition, needle: string): boolean {
  if (!needle) return true;
  const label = tOr(`widget.types.${widget.type}`, widget.label);
  return (
    label.toLowerCase().includes(needle) ||
    widget.type.includes(needle) ||
    widget.description.toLowerCase().includes(needle)
  );
}
