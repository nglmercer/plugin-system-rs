import { h, VNode } from "preact";
import { useState } from "preact/hooks";
import { callPluginCommand, fetchPluginData } from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import "./pluginData.css";

/**
 * Renders any plugin's `interface-data` without knowing anything about it.
 *
 * This is the answer to "how do I show my plugin on the dashboard?" for a
 * plugin author who has not written a line of TypeScript. Every plugin already
 * exposes a JSON blob through `interface-data` and accepts JSON commands, which
 * is enough to render a readable panel and a few buttons. A plugin only needs a
 * bespoke widget once it wants a bespoke *interaction*.
 *
 * The rendering is deliberately shallow and non-clever: scalars become rows,
 * arrays of objects become small tables, and anything deeper is shown as JSON.
 * Guessing harder would produce a layout that breaks the moment a plugin
 * changes its payload.
 */
export function PluginDataWidget({ settings }: { settings: Record<string, any> }) {
  const pluginName = String(settings.pluginName || "");
  const path = String(settings.dataPath || "");
  const refreshInterval = Number(settings.refreshInterval) || 2000;
  const actions = parseActions(settings.actions);

  const [data, setData] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  usePolling(async () => {
    if (!pluginName) return;
    try {
      const response = await fetchPluginData(pluginName);
      if (!response) throw new Error(`Plugin "${pluginName}" is not loaded`);
      setData(resolvePath(response.data, path));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      throw e;
    }
  }, refreshInterval);

  async function runAction(method: string, args: Record<string, any>) {
    setBusy(true);
    try {
      await callPluginCommand(pluginName, method, args);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!pluginName) {
    return h("div", { class: "plugin-widget empty" }, "Pick a plugin in the widget settings.");
  }

  return h(
    "div",
    { class: "plugin-widget" },
    error ? h("div", { class: "plugin-widget-error" }, error) : null,
    data === null && !error
      ? h("div", { class: "plugin-widget-empty" }, "Waiting for data...")
      : renderValue(data),
    actions.length > 0
      ? h(
          "div",
          { class: "plugin-widget-actions" },
          actions.map((action) =>
            h(
              "button",
              {
                key: action.label,
                class: "plugin-widget-btn",
                disabled: busy,
                onClick: () => runAction(action.method, action.args),
              },
              action.label,
            ),
          ),
        )
      : null,
  );
}

/** Walk a dotted path, tolerating array indices. Empty path returns the root. */
function resolvePath(value: any, path: string): any {
  if (!path) return value;
  return path.split(".").reduce((current, key) => {
    if (current === null || current === undefined) return undefined;
    return current[key];
  }, value);
}

function renderValue(value: any): VNode<any> {
  if (value === null || value === undefined) {
    return h("div", { class: "plugin-widget-empty" }, "No data");
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return h("div", { class: "plugin-widget-empty" }, "Empty list");
    }
    // An array of flat objects is a table; anything else is a plain list.
    if (value.every((entry) => isFlatObject(entry))) {
      return renderTable(value);
    }
    return h(
      "div",
      { class: "plugin-widget-list" },
      value.map((entry, index) =>
        h("div", { class: "plugin-widget-row", key: index }, formatScalar(entry)),
      ),
    );
  }

  if (typeof value === "object") {
    return h(
      "div",
      { class: "plugin-widget-list" },
      Object.entries(value).map(([key, entry]) =>
        h(
          "div",
          { class: "plugin-widget-row", key },
          h("span", { class: "plugin-widget-key" }, humanize(key)),
          h(
            "span",
            { class: "plugin-widget-value" },
            isScalar(entry) ? formatScalar(entry) : formatScalar(JSON.stringify(entry)),
          ),
        ),
      ),
    );
  }

  return h("div", { class: "plugin-widget-scalar" }, formatScalar(value));
}

function renderTable(rows: Record<string, any>[]): VNode<any> {
  // Union of keys, so a row missing a field still lines up with the rest.
  const columns = [...new Set(rows.flatMap((row) => Object.keys(row)))];

  return h(
    "div",
    { class: "plugin-widget-table-wrap" },
    h(
      "table",
      { class: "plugin-widget-table" },
      h(
        "thead",
        null,
        h(
          "tr",
          null,
          columns.map((column) => h("th", { key: column }, humanize(column))),
        ),
      ),
      h(
        "tbody",
        null,
        rows.map((row, index) =>
          h(
            "tr",
            { key: index },
            columns.map((column) => h("td", { key: column }, formatScalar(row[column]))),
          ),
        ),
      ),
    ),
  );
}

function isScalar(value: any): boolean {
  return value === null || ["string", "number", "boolean"].includes(typeof value);
}

function isFlatObject(value: any): boolean {
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.values(value).every(isScalar)
  );
}

function formatScalar(value: any): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "boolean") return value ? "Yes" : "No";
  if (typeof value === "number") {
    // Long floats are noise on a dashboard tile.
    return Number.isInteger(value) ? String(value) : value.toFixed(2);
  }
  return String(value);
}

/** `master_volume` → `Master volume`. */
function humanize(key: string): string {
  const spaced = key.replace(/[_-]/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1).toLowerCase();
}

export interface PluginAction {
  label: string;
  method: string;
  args: Record<string, any>;
}

/**
 * Parse the action list from settings.
 *
 * Stored as JSON text because it comes from a key/value editor. A malformed
 * list yields no buttons rather than throwing — the data panel above it is
 * still worth showing.
 */
export function parseActions(raw: any): PluginAction[] {
  if (!raw || typeof raw !== "string" || !raw.trim()) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return [];
    return Object.entries(parsed).map(([label, spec]) => {
      // Either `"Start": "start"` or `"Start": {"method": "start", "args": {}}`.
      if (typeof spec === "string") return { label, method: spec, args: {} };
      const object = spec as { method?: string; args?: Record<string, any> };
      return { label, method: object.method || label, args: object.args || {} };
    });
  } catch {
    return [];
  }
}
