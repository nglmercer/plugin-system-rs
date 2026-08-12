import { h } from "preact";
import { ObsInputsWidget } from "../../components/widgets/ObsInputsWidget";
import { Icons } from "../../ui/icons/Icons";
import { bar, box, chip, line, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const obsInputsWidget: WidgetDefinition = {
  type: "obs-inputs",
  label: "OBS Inputs",
  description: "Input volume and mute",
  icon: Icons.obsInputs,
  category: "obs",
  requires: [requiresPlugin("obs", "Reads and sets OBS input levels.")],
  defaultSize: { colSpan: 1, rowSpan: 2 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Input count with mute toggles",
      preview: () =>
        box(
          "obsinput-minimal",
          text("3 inputs"),
          h("div", { class: "mini-list" }, line(h("span", null, "Mic"), chip("M"))),
        ),
    },
    {
      value: "compact",
      label: "Compact",
      description: "List with sliders and mute",
      preview: () => box("obsinput-compact", text("Mic"), bar(75)),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full input controls with kind",
      preview: () => box("obsinput-detailed", text("Mic (audio)"), bar(75), text("75%")),
    },
  ],
  settings: [
    {
      kind: "interval",
      key: "refreshInterval",
      label: "Refresh interval",
      hint: "ms",
      default: 2000,
      min: 500,
    },
  ],
  component: ObsInputsWidget,
};
