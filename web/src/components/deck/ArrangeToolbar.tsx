import { h } from "preact";
import { Icons } from "../../ui/icons/Icons";
import { t } from "../../lib/i18n";

/**
 * Arrange-mode controls, shown above the deck only while arranging.
 *
 * Entering and leaving arrange mode lives in the FAB menu, so this bar keeps
 * just the controls that matter mid-arrangement: the grid steppers and Done.
 * Add-widget is in the FAB menu as well — it used to sit here too, duplicated.
 */

interface StepperProps {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (next: number) => void;
}

function Stepper({ label, value, min, max, onChange }: StepperProps) {
  return h(
    "div",
    { class: "deck-toolbar-group" },
    h("label", null, label),
    h(
      "div",
      { class: "deck-stepper" },
      h(
        "button",
        {
          onClick: () => onChange(value - 1),
          disabled: value <= min,
          "aria-label": `${label} -`,
        },
        "-",
      ),
      h("span", null, String(value)),
      h(
        "button",
        {
          onClick: () => onChange(value + 1),
          disabled: value >= max,
          "aria-label": `${label} +`,
        },
        "+",
      ),
    ),
  );
}

export interface ArrangeToolbarProps {
  columns: number;
  rows: number;
  onGridChange: (patch: { columns?: number; rows?: number }) => void;
  onDone: () => void;
}

export function ArrangeToolbar({ columns, rows, onGridChange, onDone }: ArrangeToolbarProps) {
  return h(
    "div",
    { class: "arrange-toolbar" },
    h(Stepper, {
      label: t("dashboard.columns"),
      value: columns,
      min: 1,
      max: 12,
      onChange: (v) => onGridChange({ columns: v }),
    }),
    h(Stepper, {
      label: t("dashboard.rows"),
      value: rows,
      min: 1,
      max: 12,
      onChange: (v) => onGridChange({ rows: v }),
    }),
    h(
      "button",
      { class: "btn btn-primary arrange-done", onClick: onDone },
      h(Icons.check, null),
      t("dashboard.done"),
    ),
  );
}
