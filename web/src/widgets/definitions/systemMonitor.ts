import { h } from "preact";
import { SystemMonitorWidget } from "../../components/widgets/SystemMonitorWidget";
import { Icons } from "../../ui/icons/Icons";
import { bar, box, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const systemMonitorWidget: WidgetDefinition = {
  type: "system-monitor",
  label: "System Monitor",
  description: "CPU, memory, load and uptime",
  icon: Icons.systemMonitor,
  category: "system",
  requires: [
    requiresPlugin("system-monitor", "Reads CPU, memory and process counts from the host."),
  ],
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Just CPU% and RAM% numbers",
      preview: () => box("sysmon-minimal", text("42% CPU"), text("56% RAM")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "CPU and RAM bars with load",
      preview: () => box("sysmon-compact", bar(42), bar(56, "var(--sd-accent)")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full stats with swap, cores and uptime",
      preview: () =>
        box(
          "sysmon-detailed",
          h(
            "div",
            { class: "mini-grid" },
            text("42%"),
            text("56%"),
            text("1.2"),
            text("2d"),
          ),
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
  component: SystemMonitorWidget,
};
