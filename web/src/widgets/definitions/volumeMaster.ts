import { VolumeWidget } from "../../components/widgets/VolumeWidget";
import { Icons } from "../../ui/icons/Icons";
import { bar, box, chip, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const volumeMasterWidget: WidgetDefinition = {
  type: "volume-master",
  label: "Volume Control",
  description: "Master volume and mute",
  icon: Icons.volume,
  category: "audio",
  requires: [
    requiresPlugin("volume-master", "Reads and sets the system output volume."),
  ],
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Volume percentage and a mute button",
      preview: () => box("vol-minimal", text("75%"), chip("MUTE")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Slider with the device name",
      preview: () => box("vol-compact", bar(75), text("Speaker")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full controls with per-app volume",
      preview: () => box("vol-detailed", text("75%"), bar(75), text("Apps: 2", "mini-apps")),
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
  component: VolumeWidget,
};
