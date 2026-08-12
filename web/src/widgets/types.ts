/**
 * What a widget is, declaratively.
 *
 * Adding a widget used to mean editing seven files that each held one column
 * of the same table: the catalog in `widgetHelpers`, the variant list in
 * `types.ts`, a `switch` in `WidgetContent`, an `if` chain in `WizardConfig`, a
 * `switch` in `WizardStyle`, an icon map in `Icons`, and three locale files.
 * Miss one and the widget half-exists — which is exactly how the timer plugin
 * ended up shipping with no way to reach it.
 *
 * A `WidgetDefinition` is that whole row in one place. Everything the app needs
 * to list, configure, validate, preview and render a widget is reachable from
 * this object, so a new widget is one new file plus one import.
 */

import { ComponentType, VNode } from "preact";

/** Groups the widget library shows as sections. */
export type WidgetCategory =
  | "system"
  | "time"
  | "input"
  | "audio"
  | "obs"
  | "web"
  | "plugin";

export const WIDGET_CATEGORIES: WidgetCategory[] = [
  "system",
  "time",
  "input",
  "audio",
  "obs",
  "web",
  "plugin",
];

/* ── Requirements ──────────────────────────────────────────────────────────
 *
 * A widget that needs something from the host says so, instead of rendering an
 * error and leaving the user to guess. `ObsWidget` showing a bare "not
 * connected to OBS" is the symptom this exists to fix: the widget could not
 * distinguish "the OBS plugin is not installed" from "OBS itself is closed",
 * and told the user neither.
 */

export type Requirement = {
  kind: "plugin";
  /** Plugin name as `/api/plugins` reports it. */
  plugin: string;
  /** Why this widget needs it, shown to the user. */
  reason: string;
};

/** Convenience constructor, so definitions read as prose. */
export function requiresPlugin(plugin: string, reason: string): Requirement {
  return { kind: "plugin", plugin, reason };
}

/* ── Settings schema ─────────────────────────────────────────────────────── */

interface FieldBase {
  /** Key in the widget's `settings` object. */
  key: string;
  label: string;
  /** Short note rendered beside the label. */
  hint?: string;
  /**
   * Blocks the wizard from advancing while empty. This is the "requirement"
   * half of a config step — a Fetch widget with no URL is not configured, and
   * letting it be saved just moves the failure to the dashboard.
   */
  required?: boolean;
  /** Hide the field unless the rest of the settings call for it. */
  visibleWhen?: (settings: Record<string, any>) => boolean;
  /** Extra validation beyond `required`. Return a message to reject. */
  validate?: (value: any, settings: Record<string, any>) => string | null;
}

export type SettingsField =
  | (FieldBase & { kind: "text"; default: string; placeholder?: string })
  | (FieldBase & { kind: "textarea"; default: string; placeholder?: string; rows?: number })
  | (FieldBase & { kind: "url"; default: string; placeholder?: string })
  | (FieldBase & { kind: "password"; default: string; placeholder?: string })
  | (FieldBase & { kind: "number"; default: number; min?: number; max?: number; step?: number })
  | (FieldBase & { kind: "interval"; default: number; min?: number })
  | (FieldBase & { kind: "select"; default: string; options: { value: string; label: string }[] })
  | (FieldBase & { kind: "toggle"; default: boolean })
  | (FieldBase & { kind: "hotkey"; default: string })
  | (FieldBase & { kind: "keyvalue"; default: string; placeholder?: { key: string; value: string } })
  /**
   * Escape hatch for a control the schema cannot express. Kept deliberately
   * narrow: everything shipped today fits a declarative field, and a definition
   * reaching for this is a hint that the schema is missing a kind.
   */
  | (FieldBase & {
      kind: "custom";
      default: any;
      render: (props: {
        value: any;
        settings: Record<string, any>;
        onChange: (value: any) => void;
      }) => VNode<any>;
    });

/* ── Variants ────────────────────────────────────────────────────────────── */

export interface WidgetVariant {
  value: string;
  label: string;
  description: string;
  /**
   * Miniature shown in the style step. Lives with the variant it illustrates,
   * so a new variant cannot be added without one — the previous `switch` in
   * `WizardStyle` silently fell through to a generic "Action" box instead.
   */
  preview?: () => VNode<any>;
}

/* ── The definition ──────────────────────────────────────────────────────── */

export interface WidgetViewProps {
  settings: Record<string, any>;
}

export interface WidgetDefinition {
  /** Stable id, stored in saved layouts. Never rename one in place. */
  type: string;
  /** Fallback label. `widget.types.<type>` in the locale files wins. */
  label: string;
  description: string;
  icon: ComponentType;
  category: WidgetCategory;
  /** What must be true of the host for this widget to work. */
  requires?: Requirement[];
  defaultSize: { colSpan: number; rowSpan: number };
  variants?: WidgetVariant[];
  /** Falls back to the first variant. */
  defaultVariant?: string;
  settings?: SettingsField[];
  component: ComponentType<WidgetViewProps>;
  /**
   * Where the definition came from. `external` widgets were registered at
   * runtime by a plugin's script and are labelled as such in the library, so a
   * user can tell shipped widgets from contributed ones.
   */
  source?: "builtin" | "external";
}

/** Every field's default, which is also the shape a new widget starts with. */
export function defaultSettings(definition: WidgetDefinition): Record<string, any> {
  const settings: Record<string, any> = {};
  for (const field of definition.settings ?? []) {
    settings[field.key] = field.default;
  }
  const variant = definition.defaultVariant ?? definition.variants?.[0]?.value;
  if (variant) settings.variant = variant;
  return settings;
}

/** Fields currently applicable, honouring `visibleWhen`. */
export function visibleFields(
  definition: WidgetDefinition,
  settings: Record<string, any>,
): SettingsField[] {
  return (definition.settings ?? []).filter(
    (field) => !field.visibleWhen || field.visibleWhen(settings),
  );
}

/**
 * Problems that should stop the wizard advancing, keyed by field.
 *
 * Only visible fields are checked: a hidden field's value is irrelevant by
 * definition, and rejecting on one the user cannot see is a dead end.
 */
export function validateSettings(
  definition: WidgetDefinition,
  settings: Record<string, any>,
): Record<string, string> {
  const errors: Record<string, string> = {};

  for (const field of visibleFields(definition, settings)) {
    const value = settings[field.key];

    if (field.required && isBlank(value)) {
      errors[field.key] = `${field.label} is required`;
      continue;
    }
    // A blank optional field is fine and must not be handed to `validate`,
    // which is written to judge a value the user actually supplied.
    if (isBlank(value)) continue;

    const message = field.validate?.(value, settings);
    if (message) errors[field.key] = message;
  }

  return errors;
}

function isBlank(value: any): boolean {
  return value === undefined || value === null || value === "";
}
