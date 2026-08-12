/**
 * The one table of known widgets.
 *
 * Built-ins register themselves at import time (see `widgets/index.ts`);
 * plugin-contributed widgets register at runtime through the same door. Nothing
 * else in the app is allowed to keep its own list of widget types — the point
 * of this module is that there is exactly one, and that a lookup for an unknown
 * type fails visibly rather than falling through a `default:` branch to
 * "Unknown widget".
 */

import { WidgetCategory, WidgetDefinition, WIDGET_CATEGORIES } from "./types";

const registry = new Map<string, WidgetDefinition>();
const listeners = new Set<() => void>();

/**
 * Add a widget definition.
 *
 * Re-registering a type replaces it, which is what makes hot module reload and
 * a plugin overriding a built-in both work. It is logged, because a silent
 * replacement is otherwise very hard to explain when two plugins claim the same
 * type.
 */
export function registerWidget(definition: WidgetDefinition): void {
  if (registry.has(definition.type)) {
    console.warn(
      `[widgets] "${definition.type}" is already registered; replacing it. ` +
        `Two definitions claiming one type usually means a plugin picked a ` +
        `name a built-in widget already uses.`,
    );
  }
  registry.set(definition.type, { source: "builtin", ...definition });
  notify();
}

/** Register several at once. */
export function registerWidgets(definitions: WidgetDefinition[]): void {
  definitions.forEach(registerWidget);
}

/** Remove a definition. Used when a plugin providing it is disabled. */
export function unregisterWidget(type: string): boolean {
  const removed = registry.delete(type);
  if (removed) notify();
  return removed;
}

export function getWidget(type: string): WidgetDefinition | undefined {
  return registry.get(type);
}

export function hasWidget(type: string): boolean {
  return registry.has(type);
}

/** Every definition, in registration order. */
export function listWidgets(): WidgetDefinition[] {
  return [...registry.values()];
}

export function listWidgetTypes(): string[] {
  return [...registry.keys()];
}

/** Definitions grouped for the library, skipping categories with nothing in them. */
export function widgetsByCategory(): { category: WidgetCategory; widgets: WidgetDefinition[] }[] {
  const grouped = WIDGET_CATEGORIES.map((category) => ({
    category,
    widgets: listWidgets().filter((widget) => widget.category === category),
  })).filter((group) => group.widgets.length > 0);

  // A definition with a category outside the known set still has to be
  // reachable — dropping it would make a plugin's widget silently invisible.
  // It joins the existing "plugin" section rather than starting a second one
  // with the same heading.
  const known = new Set<string>(WIDGET_CATEGORIES);
  const orphans = listWidgets().filter((widget) => !known.has(widget.category));
  if (orphans.length > 0) {
    const plugins = grouped.find((group) => group.category === "plugin");
    if (plugins) {
      plugins.widgets = [...plugins.widgets, ...orphans];
    } else {
      grouped.push({ category: "plugin", widgets: orphans });
    }
  }

  return grouped;
}

/* ── Change notification ───────────────────────────────────────────────────
 *
 * The library and the dashboard both render from the registry, and a plugin can
 * add to it after they have mounted. `useWidgetRegistry` subscribes so those
 * views pick up a late registration instead of needing a page reload.
 */

let version = 0;

export function subscribeToRegistry(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function notify(): void {
  version += 1;
  listeners.forEach((listener) => listener());
}

/** Bumped on every change, so a view can use it as a render key. */
export function registryVersion(): number {
  return version;
}
