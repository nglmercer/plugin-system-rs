import { h } from "preact";
import { ObsMediaWidget } from "../../components/widgets/ObsMediaWidget";
import { Icons } from "../../ui/icons/Icons";
import { bar, box, chips, line, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const obsMediaWidget: WidgetDefinition = {
  type: "obs-media",
  label: "OBS Media",
  description: "Play, pause and stop media sources",
  icon: Icons.obsMedia,
  category: "obs",
  requires: [requiresPlugin("obs", "Lists and controls OBS media sources.")],
  defaultSize: { colSpan: 1, rowSpan: 2 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "The first media source with transport buttons",
      preview: () => box("obsmedia-minimal", text("Stinger"), text("playing"), chips("▶", "■")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Every media source with play and stop",
      preview: () =>
        box(
          "obsmedia-compact",
          h("div", { class: "mini-list" }, line(text("Stinger"), chips("▶", "■"))),
        ),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Adds restart and a progress bar",
      preview: () =>
        box("obsmedia-detailed", line(text("Stinger"), chips("▶", "■", "↺")), bar(45)),
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
  component: ObsMediaWidget,
};
