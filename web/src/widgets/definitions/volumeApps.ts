import { VolumeAppsWidget } from "../../components/VolumeAppsWidget";
import { Icons } from "../../ui/icons/Icons";
import { bar, box, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const volumeAppsWidget: WidgetDefinition = {
  type: "volume-apps",
  label: "App Volume",
  description: "Per-application volume control",
  icon: Icons.volumeApps,
  category: "audio",
  requires: [
    requiresPlugin("volume-master", "Lists application audio streams and sets their volume."),
  ],
  defaultSize: { colSpan: 2, rowSpan: 2 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "App count and a mini list",
      preview: () => box("volapps-minimal", text("3 apps"), text("Firefox, Spotify", "mini-list")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "List with sliders",
      preview: () => box("volapps-compact", text("Firefox"), bar(60)),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full per-app controls",
      preview: () =>
        box("volapps-detailed", text("Firefox (PID: 1234)"), bar(60), text("60%")),
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
  component: VolumeAppsWidget,
};
