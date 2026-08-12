import { h } from "preact";
import { PluginDataWidget } from "../../components/PluginDataWidget";
import { FormSelect } from "../../components/FormComponents";
import { Icons } from "../../ui/icons/Icons";
import { box, chips, line, text } from "../preview";
import { WidgetDefinition } from "../types";
import { hostStatusSnapshot } from "../useHostStatus";

/**
 * A widget for a plugin that has no widget of its own.
 *
 * Every plugin exposes `interface-data` and accepts JSON commands, so this can
 * render one usefully without any plugin-specific frontend code. That is the
 * point: installing a plugin should not require a matching dashboard release.
 * A plugin wanting something richer ships its own definition — see
 * `widgets/external.ts`.
 */
export const pluginDataWidget: WidgetDefinition = {
  type: "plugin-data",
  label: "Plugin Panel",
  description: "Show any plugin's data, and call its commands",
  icon: Icons.puzzle,
  category: "plugin",
  defaultSize: { colSpan: 1, rowSpan: 1 },
  defaultVariant: "compact",
  variants: [
    {
      value: "compact",
      label: "Panel",
      description: "Key/value rows, tables for lists",
      preview: () =>
        box(
          "plugin-preview",
          line(h("span", null, "state"), h("span", null, "ready")),
          line(h("span", null, "count"), h("span", null, "3")),
        ),
    },
    {
      value: "detailed",
      label: "Panel + actions",
      description: "Adds buttons that call plugin commands",
      preview: () =>
        box(
          "plugin-preview",
          line(h("span", null, "state"), h("span", null, "ready")),
          chips("Start", "Stop"),
        ),
    },
  ],
  settings: [
    {
      kind: "custom",
      key: "pluginName",
      label: "Plugin",
      hint: "Loaded plugins only",
      default: "",
      required: true,
      // A live list beats a free-text box: the set of valid answers is known,
      // and typing one wrong produces a widget that just says "not loaded".
      render: ({ value, onChange }) => {
        const plugins = hostStatusSnapshot().plugins.filter((p) => p.loaded);
        return h(FormSelect, {
          value: String(value ?? ""),
          options: [
            { value: "", label: plugins.length ? "Choose a plugin..." : "No plugins loaded" },
            ...plugins.map((p) => ({ value: p.name, label: p.name })),
          ],
          onChange,
        });
      },
    },
    {
      kind: "text",
      key: "dataPath",
      label: "Data path",
      hint: "Dotted path into the plugin's data. Blank shows all of it.",
      default: "",
      placeholder: "timers",
    },
    {
      kind: "keyvalue",
      key: "actions",
      label: "Action buttons",
      hint: "Label → command method",
      default: "",
      placeholder: { key: "Refresh", value: "refresh" },
      visibleWhen: (settings) => settings.variant === "detailed",
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
  component: PluginDataWidget,
};
