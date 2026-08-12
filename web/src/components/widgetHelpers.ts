/**
 * Widget helpers, now thin wrappers over the registry.
 *
 * This file used to hold `WIDGET_CATALOG`, `getDefaultVariant` and
 * `getDefaultSettings` — three hand-maintained tables keyed by widget type that
 * had to agree with each other and with four other files. They are gone; the
 * definitions carry that data. What is left is the couple of functions callers
 * genuinely share.
 */

import { WidgetConfig } from "../lib/types";
import { defaultSettings, getWidget } from "../widgets";

export function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

/** Whether the wizard has a config step for this type. */
export function widgetHasConfig(type: string): boolean {
  return (getWidget(type)?.settings?.length ?? 0) > 0;
}

/**
 * A new widget of the given type, at its declared default size.
 *
 * Throws for an unregistered type rather than inventing a placeholder: a caller
 * asking for a widget that does not exist has a bug, and a silently
 * half-constructed widget lands in the saved layout where it is much harder to
 * trace.
 */
export function buildDefaultWidget(type: string): WidgetConfig {
  const definition = getWidget(type);
  if (!definition) {
    throw new Error(`Unknown widget type "${type}"`);
  }

  return {
    id: generateId(),
    type: definition.type,
    title: definition.label,
    colSpan: definition.defaultSize.colSpan,
    rowSpan: definition.defaultSize.rowSpan,
    settings: defaultSettings(definition),
  };
}
