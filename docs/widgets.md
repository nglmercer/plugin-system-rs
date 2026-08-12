# Widgets

A widget is a tile on the dashboard. This document covers the three ways to get
one, from least to most work.

The dashboard reads everything it knows about a widget from a single
`WidgetDefinition`. There is no separate catalog, no render `switch`, no config
`if` chain and no icon map to keep in step — those existed once, and a widget
that was in some of them but not others was the most common bug in this part of
the codebase.

## 1. No frontend code: the Plugin Panel

Every plugin already exposes `interface-data` and accepts JSON commands. The
built-in **Plugin Panel** widget renders that directly:

1. Add a *Plugin Panel* widget from the library.
2. Pick your plugin from the dropdown (only loaded plugins are listed).
3. Optionally set a **data path** to drill into the payload — `timers` shows
   just that key.
4. Choose the *Panel + actions* variant to add buttons, each mapping a label to
   a command method.

Scalars become rows, arrays of flat objects become tables, and anything deeper
is shown as JSON. That is enough for most plugins, and it costs nothing to keep
working when your payload changes.

## 2. A built-in widget

One file under `web/src/widgets/definitions/`, and one line in
`web/src/widgets/index.ts`. Nothing else.

```ts
import { MyWidget } from "../../components/MyWidget";
import { Icons } from "../../ui/icons/Icons";
import { box, text } from "../preview";
import { WidgetDefinition, requiresPlugin } from "../types";

export const myWidget: WidgetDefinition = {
  type: "my-widget",              // stable id; stored in saved layouts
  label: "My Widget",
  description: "What it does, in a few words",
  icon: Icons.systemMonitor,
  category: "system",

  // Declares what the host must provide. Drives the wizard's Requires step,
  // the library's unavailable badge, and the placeholder shown in place of a
  // widget that cannot work.
  requires: [requiresPlugin("my-plugin", "Reads the thing this widget shows.")],

  defaultSize: { colSpan: 1, rowSpan: 1 },

  // Omit for a widget with one look. A single variant produces no Style step.
  variants: [
    {
      value: "compact",
      label: "Compact",
      description: "One line",
      preview: () => box("my-compact", text("42")),
    },
  ],

  // Omit for a widget with nothing to configure — it then gets no Config step
  // rather than an empty one.
  settings: [
    {
      kind: "text",
      key: "name",
      label: "Name",
      default: "",
      required: true,        // blocks Next until filled
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

  component: MyWidget,       // receives { settings }
};
```

Then add it to `BUILTIN_WIDGETS` in `web/src/widgets/index.ts`.

### Setting field kinds

`text`, `textarea`, `url`, `password`, `number`, `interval`, `select`,
`toggle`, `hotkey`, `keyvalue`, and `custom` for a control the schema cannot
express. Every field supports:

| Property | Effect |
|---|---|
| `required` | Blocks the wizard while blank |
| `validate(value, settings)` | Return a message to reject; only called on a supplied value |
| `visibleWhen(settings)` | Hides the field, and exempts it from validation |
| `hint` | Small note beside the label |

**The keys must match what your component reads.** A schema field named
`refreshInterval` against a component reading `intervalSec` produces a control
that silently changes nothing — which is exactly the drift this system exists to
prevent, so it is worth a second look.

## 3. A widget from outside the bundle

A plugin can register a full definition at runtime. The page exposes
`window.streamdeck` before any plugin script runs:

```js
const { h, hooks, api, registerWidget } = window.streamdeck;

function TemperatureWidget({ settings }) {
  const [value, setValue] = hooks.useState(null);

  hooks.useEffect(() => {
    const id = setInterval(async () => {
      const data = await api.fetchPluginData("thermostat");
      setValue(data?.data?.celsius ?? null);
    }, settings.refreshInterval || 5000);
    return () => clearInterval(id);
  }, [settings.refreshInterval]);

  return h("div", { class: "my-temp" }, value === null ? "—" : `${value} °C`);
}

registerWidget({
  type: "thermostat-temp",
  label: "Temperature",
  description: "Current room temperature",
  component: TemperatureWidget,
  requires: [{ kind: "plugin", plugin: "thermostat", reason: "Reads the sensor." }],
  settings: [
    { kind: "interval", key: "refreshInterval", label: "Refresh", default: 5000, min: 1000 },
  ],
});
```

Notes:

- Use `window.streamdeck.h`, not your own copy of Preact. A VNode from a
  different renderer instance will not mount.
- Use `window.streamdeck.api` for requests. It goes through the authenticated
  client, so your script never handles the API token.
- `registerWidget` returns a function that unregisters the widget again.
- A malformed definition is rejected with a console error naming the problem,
  rather than crashing the grid later.
- `category` defaults to `plugin`, and a widget with no `icon` gets its type's
  initial. `source` is set to `external` and the library labels it as coming
  from a plugin.
- The surface is versioned as `window.streamdeck.version`; check it if you
  support several dashboard releases.

## Wizard steps

Steps are derived from the definition, so there are never empty ones:

| Step | Appears when |
|---|---|
| **Requires** | `requires` is non-empty |
| **General** | Always — title and grid footprint |
| **Config** | `settings` is non-empty |
| **Style** | `variants` has more than one entry |
| **Apply** | Always — review, preview, delete |

Requirements never block: someone laying out a dashboard before starting OBS is
doing something reasonable. Invalid *settings* do block, because a widget saved
without its required configuration just moves the failure to the dashboard.
