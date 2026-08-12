import { TimerWidget } from "../../components/widgets/TimerWidget";
import { Icons } from "../../ui/icons/Icons";
import { bar, box, chips, line, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";
import { h } from "preact";

export const timerWidget: WidgetDefinition = {
  type: "timer",
  label: "Timer",
  description: "Countdown timers",
  icon: Icons.timer,
  category: "time",
  requires: [requiresPlugin("timer", "Holds the countdown, so it survives a page reload.")],
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Just the remaining time",
      preview: () => box("timer-minimal", text("4:32", "mini-time")),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Countdown with start and stop",
      preview: () => box("timer-compact", text("4:32", "mini-time"), chips("Start", "Stop")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "All running timers with progress",
      preview: () =>
        box(
          "timer-detailed",
          line(h("span", null, "break"), h("span", null, "4:32")),
          bar(40, "var(--sd-accent)"),
          line(h("span", null, "pomodoro"), h("span", null, "0:00")),
        ),
    },
  ],
  settings: [
    {
      kind: "text",
      key: "timerName",
      label: "Timer name",
      hint: "Widgets sharing a name share the countdown",
      default: "timer",
      placeholder: "pomodoro",
      required: true,
    },
    {
      kind: "number",
      key: "seconds",
      label: "Duration",
      hint: "seconds",
      default: 300,
      min: 1,
      required: true,
    },
    {
      kind: "interval",
      key: "refreshInterval",
      label: "Refresh interval",
      // Below a second the displayed countdown visibly skips.
      hint: "ms — 1000 keeps the seconds smooth",
      default: 1000,
      min: 250,
    },
  ],
  component: TimerWidget,
};
