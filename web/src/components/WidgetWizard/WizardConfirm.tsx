import { h } from "preact";
import { t } from "../../lib/i18n";
import { WidgetConfig } from "../../lib/types";
import { RequirementReport, WidgetDefinition, visibleFields } from "../../widgets";
import { WidgetContent } from "../WidgetContent";

/**
 * Review and save.
 *
 * The summary reads from the schema, so a row is labelled "Refresh interval"
 * rather than "refreshInterval" and a password is not printed into the page.
 * The old version dumped `Object.entries(settings)` verbatim, which meant every
 * new setting appeared here as a raw key whether or not it made sense to show.
 */
export function WizardConfirm({
  definition,
  widget,
  title,
  colSpan,
  rowSpan,
  settings,
  variant,
  requirements,
  onRemove,
}: {
  definition: WidgetDefinition;
  widget: WidgetConfig;
  title: string;
  colSpan: number;
  rowSpan: number;
  settings: Record<string, any>;
  variant: string;
  requirements: RequirementReport;
  onRemove: () => void;
}) {
  const variantLabel =
    definition.variants?.find((entry) => entry.value === variant)?.label ?? variant;

  const rows: [string, string][] = [
    [t("widget.wizard.title"), title],
    [t("widget.wizard.size"), `${colSpan} × ${rowSpan}`],
  ];
  if (definition.variants?.length) {
    rows.push([t("widget.wizard.variant"), variantLabel]);
  }
  for (const field of visibleFields(definition, settings)) {
    rows.push([field.label, displayValue(field.kind, settings[field.key])]);
  }

  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, t("widget.wizard.apply")),
    h("p", { class: "wizard-step-desc" }, t("widget.wizard.applyDesc")),

    requirements.satisfied
      ? null
      : h(
          "div",
          { class: "wizard-warning" },
          h("strong", null, t("widget.wizard.requirementsMissingShort")),
          " ",
          requirements.unmet
            .map((result) => `${result.requirement.plugin} — ${result.status}`)
            .join("; "),
        ),

    h(
      "div",
      { class: "confirm-details" },
      rows.map(([label, value]) =>
        h(
          "div",
          { class: "confirm-row", key: label },
          h("span", null, label),
          h("span", null, value),
        ),
      ),
    ),

    h(
      "div",
      { class: "confirm-preview" },
      h("div", { class: "confirm-preview-label" }, t("widget.wizard.preview")),
      h(
        "div",
        { class: "preview-frame" },
        h(WidgetContent, {
          widget: {
            ...widget,
            title,
            colSpan,
            rowSpan,
            settings: { ...settings, variant },
          },
        }),
      ),
    ),

    h(
      "button",
      { class: "wizard-remove-btn", type: "button", onClick: onRemove },
      t("widget.wizard.delete"),
    ),
  );
}

/** Render a setting for the summary, keeping secrets out of it. */
function displayValue(kind: string, value: any): string {
  if (value === undefined || value === null || value === "") return "—";
  // Printing a password into a review screen is a small betrayal of the person
  // who typed it into a masked box one step earlier.
  if (kind === "password") return "••••••••";
  if (typeof value === "boolean") return value ? t("common.yes") : t("common.no");
  const text = String(value);
  return text.length > 60 ? `${text.slice(0, 60)}…` : text;
}
