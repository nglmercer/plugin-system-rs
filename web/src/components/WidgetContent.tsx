import { h } from "preact";
import { WidgetConfig } from "../lib/types";
import { t, tOr } from "../lib/i18n";
import {
  checkRequirements,
  getWidget,
  summarizeUnmet,
  useHostStatus,
} from "../widgets";

/**
 * Renders a widget's body.
 *
 * This was a `switch` over every widget type. Now it is a registry lookup, so a
 * widget that exists is rendered and a widget that does not says which type is
 * missing instead of the uniform "Unknown widget" that gave no way to tell a
 * typo from an uninstalled plugin.
 *
 * It also refuses to render a widget whose requirements are unmet, showing what
 * is missing instead. Letting it render produced the screen this refactor
 * started from: an OBS tile whose entire content was the words "not connected
 * to OBS" in red, which is true but tells the user nothing they can act on.
 */
export function WidgetContent({ widget }: { widget: WidgetConfig }) {
  const definition = getWidget(widget.type);
  const host = useHostStatus();

  if (!definition) {
    return h(
      "div",
      { class: "widget-placeholder unknown" },
      h("div", { class: "widget-placeholder-title" }, t("widget.unknownTitle")),
      h("div", { class: "widget-placeholder-detail" }, widget.type),
      h(
        "div",
        { class: "widget-placeholder-hint" },
        t("widget.unknownHint"),
      ),
    );
  }

  const requirements = checkRequirements(definition, host);
  if (!requirements.satisfied) {
    return h(
      "div",
      { class: "widget-placeholder blocked" },
      h(
        "div",
        { class: "widget-placeholder-title" },
        tOr(`widget.types.${definition.type}`, definition.label),
      ),
      h("div", { class: "widget-placeholder-detail" }, summarizeUnmet(requirements)),
      h(
        "div",
        { class: "widget-placeholder-hint" },
        requirements.unmet[0]?.remedy ?? "",
      ),
    );
  }

  return h(definition.component, { settings: widget.settings });
}
