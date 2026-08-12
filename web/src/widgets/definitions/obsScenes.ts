import { h } from "preact";
import { ObsScenesWidget } from "../../components/ObsScenesWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, chip, chips, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const obsScenesWidget: WidgetDefinition = {
  type: "obs-scenes",
  label: "OBS Scenes",
  description: "Switch between scenes",
  icon: Icons.obsScenes,
  category: "obs",
  requires: [requiresPlugin("obs", "Lists and switches OBS scenes.")],
  defaultSize: { colSpan: 1, rowSpan: 2 },
  defaultVariant: "compact",
  variants: [
    {
      value: "minimal",
      label: "Minimal",
      description: "Current scene and a button grid",
      preview: () =>
        box("obscene-minimal", text("Scene 1"), h("div", { class: "mini-grid" }, chip("S1", true), chip("S2"))),
    },
    {
      value: "compact",
      label: "Compact",
      description: "Scene list with the active one highlighted",
      preview: () =>
        box("obscene-compact", h("div", { class: "mini-list" }, chip("Scene 1", true), chip("Scene 2"))),
    },
    {
      value: "detailed",
      label: "Detailed",
      description: "Scenes, transitions and source toggles",
      preview: () =>
        box(
          "obscene-detailed",
          h("div", { class: "mini-list" }, chip("Scene 1", true), chip("Scene 2")),
          chips("Fade"),
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
  component: ObsScenesWidget,
};
