import { h } from "preact";
import { Icons } from "../ui/icons/Icons";
import { t } from "../lib/i18n";

/**
 * Dashboard header controls.
 *
 * Split out of `WidgetGrid` so that component is about layout state and this
 * one is about presenting it. The grid steppers only appear while arranging —
 * they are meaningless otherwise and were the bulk of the header's clutter.
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

export interface DeckToolbarProps {
  arranging: boolean;
  onToggleArrange: () => void;
  columns: number;
  rows: number;
  onGridChange: (patch: { columns?: number; rows?: number }) => void;
  onAddWidget: () => void;
}

export function DeckToolbar(props: DeckToolbarProps) {
  const { arranging, onToggleArrange, columns, rows, onGridChange, onAddWidget } = props;

  return h(
    "div",
    { class: "dashboard-header-actions deck-toolbar" },
    arranging &&
      h(Stepper, {
        label: t("dashboard.columns"),
        value: columns,
        min: 1,
        max: 12,
        onChange: (v) => onGridChange({ columns: v }),
      }),
    arranging &&
      h(Stepper, {
        label: t("dashboard.rows"),
        value: rows,
        min: 1,
        max: 12,
        onChange: (v) => onGridChange({ rows: v }),
      }),
    h(
      "button",
      {
        class: `deck-edit-toggle${arranging ? " is-active" : ""}`,
        onClick: onToggleArrange,
      },
      arranging ? t("dashboard.done") : t("dashboard.edit"),
    ),
    h(
      "button",
      { class: "add-widget-btn", onClick: onAddWidget },
      h(Icons.plus, null),
      t("dashboard.addWidget"),
    ),
  );
}
