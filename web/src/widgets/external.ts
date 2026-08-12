/**
 * The public door for widgets contributed from outside this bundle.
 *
 * A plugin that wants more than the generic `plugin-data` panel can ship a
 * script that registers a full definition. The script is loaded by the page and
 * calls `window.streamdeck.registerWidget(...)`; from that moment the widget is
 * indistinguishable from a built-in one — it appears in the library, gets a
 * wizard built from its schema, and has its requirements checked.
 *
 * # What an external script can and cannot do
 *
 * It cannot import from this bundle, so it cannot use `h` from our Preact copy.
 * `window.streamdeck.h` hands it the same renderer we use, which is what makes
 * a returned VNode mount correctly rather than silently producing nothing.
 *
 * A definition arriving here is untrusted input in the sense that matters for
 * robustness: it may be malformed, from a plugin written against an older
 * shape. `validateDefinition` rejects it with a specific complaint rather than
 * letting a missing field surface later as a render crash inside the grid.
 */

import { h, Fragment } from "preact";
import { useState, useEffect } from "preact/hooks";
import * as api from "../lib/api";
import { registerWidget, unregisterWidget } from "./registry";
import { WidgetDefinition } from "./types";

export interface StreamdeckWidgetApi {
  /** Add a widget. Returns a function that removes it again. */
  registerWidget: (definition: WidgetDefinition) => () => void;
  /** Our Preact `h`, so an external component mounts into our tree. */
  h: typeof h;
  Fragment: typeof Fragment;
  hooks: { useState: typeof useState; useEffect: typeof useEffect };
  /** The authenticated API client, so a script never handles the token itself. */
  api: {
    callPluginCommand: typeof api.callPluginCommand;
    fetchPluginData: typeof api.fetchPluginData;
  };
  /** Bundle contract version. Bumped when this surface changes incompatibly. */
  version: number;
}

/** Incompatible changes to the surface above bump this. */
export const EXTERNAL_API_VERSION = 1;

declare global {
  interface Window {
    streamdeck?: StreamdeckWidgetApi;
  }
}

/** Reject a definition that would break the app once rendered. */
export function validateDefinition(definition: any): string | null {
  if (!definition || typeof definition !== "object") {
    return "A widget definition must be an object";
  }
  if (typeof definition.type !== "string" || !definition.type.trim()) {
    return "A widget definition needs a non-empty string `type`";
  }
  if (typeof definition.component !== "function") {
    return `Widget "${definition.type}" needs a component function`;
  }
  if (typeof definition.label !== "string" || !definition.label.trim()) {
    return `Widget "${definition.type}" needs a label`;
  }
  if (definition.settings !== undefined && !Array.isArray(definition.settings)) {
    return `Widget "${definition.type}": settings must be an array of fields`;
  }
  if (definition.variants !== undefined && !Array.isArray(definition.variants)) {
    return `Widget "${definition.type}": variants must be an array`;
  }
  return null;
}

/** Fill in what an external definition is allowed to leave out. */
function normalize(definition: WidgetDefinition): WidgetDefinition {
  return {
    ...definition,
    category: definition.category ?? "plugin",
    description: definition.description ?? "",
    defaultSize: definition.defaultSize ?? { colSpan: 1, rowSpan: 1 },
    // The type's initial when the plugin supplies no icon: better than an
    // empty tile, and needs nothing from the plugin.
    icon:
      definition.icon ??
      (() => h("span", { class: "widget-icon-letter" }, definition.type[0].toUpperCase())),
    source: "external",
  };
}

/**
 * Expose the API on `window`.
 *
 * Called once at start-up, before any plugin script has a chance to run. Doing
 * it later would mean a script that loaded first found no `window.streamdeck`
 * and gave up.
 */
export function installExternalWidgetApi(): void {
  if (window.streamdeck) return;

  window.streamdeck = {
    version: EXTERNAL_API_VERSION,
    h,
    Fragment,
    hooks: { useState, useEffect },
    api: {
      callPluginCommand: api.callPluginCommand,
      fetchPluginData: api.fetchPluginData,
    },
    registerWidget: (definition: WidgetDefinition) => {
      const problem = validateDefinition(definition);
      if (problem) {
        console.error(`[widgets] refusing an external widget: ${problem}`);
        return () => {};
      }
      const normalized = normalize(definition);
      registerWidget(normalized);
      return () => unregisterWidget(normalized.type);
    },
  };
}
