import { h } from "preact";
import { ObsWidget } from "../../components/ObsWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, chips, dot, line, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";
import { intRange } from "../validators";

/**
 * Connection settings are shared by every OBS widget, but only this one owns
 * them: it is the widget that performs the connect, and duplicating host/port
 * across three definitions would let them disagree about where OBS is.
 */
export const obsControlWidget: WidgetDefinition = {
  type: "obs-control",
  label: "OBS Control",
  description: "Stream, record and virtual camera",
  icon: Icons.obs,
  category: "obs",
  requires: [requiresPlugin("obs", "Talks to OBS over its WebSocket API.")],
  defaultSize: { colSpan: 1, rowSpan: 2 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Status dots for stream and record",
      preview: () =>
        box(
          "obs-minimal",
          line(dot("green"), h("span", null, "Connected")),
          line(dot("red"), h("span", null, "Stream")),
        ),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Current scene and toggle buttons",
      preview: () => box("obs-compact", text("Scene 1"), chips("STR", "REC", "VC")),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Full controls with stats",
      preview: () =>
        box(
          "obs-detailed",
          chips(["Stream", true], "Record"),
          h("div", { class: "mini-grid" }, text("CPU"), text("FPS")),
        ),
    },
  ],
  settings: [
    {
      kind: "text",
      key: "host",
      label: "OBS host",
      default: "127.0.0.1",
      placeholder: "127.0.0.1",
      required: true,
    },
    {
      kind: "number",
      key: "port",
      label: "WebSocket port",
      hint: "OBS default is 4455",
      default: 4455,
      min: 1,
      max: 65535,
      required: true,
      validate: intRange(1, 65535),
    },
    {
      kind: "password",
      key: "password",
      label: "Password",
      hint: "Blank if authentication is off",
      default: "",
      placeholder: "OBS WebSocket password",
    },
    {
      kind: "toggle",
      key: "autoConnect",
      label: "Connect automatically",
      hint: "Once per load",
      default: true,
    },
    {
      kind: "interval",
      key: "refreshInterval",
      label: "Refresh interval",
      hint: "ms",
      default: 2000,
      min: 500,
    },
  ],
  component: ObsWidget,
};
