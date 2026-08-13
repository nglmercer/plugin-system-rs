import { h } from "preact";
import { ObsStudioWidget } from "../../components/widgets/ObsStudioWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, chip, chips, line, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const obsStudioWidget: WidgetDefinition = {
  type: "obs-studio",
  label: "OBS Studio Mode",
  description: "Preview, program and the transition between them",
  icon: Icons.obsStudio,
  category: "obs",
  requires: [requiresPlugin("obs", "Reads and drives OBS studio mode.")],
  defaultSize: { colSpan: 1, rowSpan: 2 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Studio toggle and the current preview → program",
      preview: () => box("obsstudio-minimal", chip("Studio ON", true), text("Intro → Live")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Toggle, scene line and the transition button",
      preview: () =>
        box("obsstudio-compact", line(text("Studio"), chip("ON", true)), text("Intro → Live"), chip("Transition")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Preview beside program, snapshot, and a scene picker",
      preview: () =>
        box(
          "obsstudio-detailed",
          line(text("Preview"), text("Program")),
          line(text("Intro"), text("Live")),
          chips("Transition", "Snapshot"),
        ),
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
  component: ObsStudioWidget,
};
