import { ClockWidget } from "../../components/ClockWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, text } from "../preview";
import { WidgetDefinition } from "../types";

/**
 * The one widget that needs nothing from the host — it reads the browser's own
 * clock. Useful as the reference for the smallest possible definition.
 */
export const clockWidget: WidgetDefinition = {
  type: "clock",
  label: "Clock",
  description: "Current time and date",
  icon: Icons.clock,
  category: "time",
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "digital",
  variants: [
    {
      value: "simple",
      label: "Simple",
      description: "Just HH:MM, no seconds",
      preview: () => box("clock-simple", "14:30"),
    },
    {
      value: "digital",
      label: "Digital",
      description: "HH:MM plus seconds and date",
      preview: () =>
        box("clock-digital", "14:30", text("15", "mini-sec"), text("Mon", "mini-date")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full date with day name",
      preview: () =>
        box("clock-detailed", "14:30:15", text("Monday, Jun 10", "mini-date")),
    },
  ],
  component: ClockWidget,
};
