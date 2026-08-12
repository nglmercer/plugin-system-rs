import { h } from "preact";
import { t } from "../../lib/i18n";
import { WidgetDefinition } from "../../widgets";

/**
 * Variant picker, rendered from the definition.
 *
 * Each variant carries its own preview, so a variant added without one is
 * obvious at the point of writing rather than falling through a `switch` to a
 * generic placeholder box the way it used to.
 */
export function WizardStyle({
  definition,
  variant,
  onChange,
}: {
  definition: WidgetDefinition;
  variant: string;
  onChange: (variant: string) => void;
}) {
  const variants = definition.variants ?? [];

  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, t("widget.wizard.style")),
    h("p", { class: "wizard-step-desc" }, t("widget.wizard.styleDesc")),
    h(
      "div",
      { class: "variant-grid" },
      variants.map((entry) =>
        h(
          "button",
          {
            class: `variant-card ${variant === entry.value ? "selected" : ""}`,
            key: entry.value,
            type: "button",
            onClick: () => onChange(entry.value),
          },
          h(
            "div",
            { class: "variant-card-preview" },
            entry.preview
              ? entry.preview()
              : h("div", { class: "variant-preview simple-preview" }, entry.label),
          ),
          h(
            "div",
            { class: "variant-card-info" },
            h("div", { class: "variant-card-label" }, entry.label),
            h("div", { class: "variant-card-desc" }, entry.description),
          ),
        ),
      ),
    ),
  );
}
