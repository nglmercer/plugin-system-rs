import { h } from "preact";
import { t } from "../../lib/i18n";
import { FormField, FormInput } from "../FormComponents";

export interface WizardGeneralValues {
  title: string;
  colSpan: number;
  rowSpan: number;
}

/**
 * Title and footprint.
 *
 * Row span is editable here now. It was always part of a widget's layout but
 * the wizard only ever offered column span, so the only way to make a widget
 * taller was to drag it — and there is no drag handle in the wizard.
 */
export function WizardGeneral({
  values,
  columns,
  rows,
  onChange,
}: {
  values: WizardGeneralValues;
  columns: number;
  rows: number;
  onChange: (values: Partial<WizardGeneralValues>) => void;
}) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, t("widget.wizard.general")),
    h(
      FormField,
      { label: t("widget.wizard.title") },
      h(FormInput, {
        value: values.title,
        placeholder: t("widget.wizard.titlePlaceholder"),
        onInput: (title) => onChange({ title }),
      }),
    ),
    h(
      "div",
      { class: "wizard-field-row" },
      h(
        FormField,
        {
          label: t("widget.wizard.columns"),
          hint: t("widget.wizard.gridHint", { count: columns }),
        },
        h(FormInput, {
          type: "number",
          value: String(values.colSpan),
          // Clamped to the grid: a widget wider than the page is a widget you
          // cannot see the right-hand half of.
          onInput: (raw) => onChange({ colSpan: clamp(raw, 1, columns) }),
        }),
      ),
      h(
        FormField,
        {
          label: t("widget.wizard.rows"),
          hint: t("widget.wizard.gridHintRows", { count: rows }),
        },
        h(FormInput, {
          type: "number",
          value: String(values.rowSpan),
          onInput: (raw) => onChange({ rowSpan: clamp(raw, 1, rows) }),
        }),
      ),
    ),
  );
}

function clamp(raw: string, min: number, max: number): number {
  const parsed = parseInt(raw, 10);
  if (Number.isNaN(parsed)) return min;
  return Math.min(max, Math.max(min, parsed));
}
