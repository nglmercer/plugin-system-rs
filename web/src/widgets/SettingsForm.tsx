import { h, Fragment, VNode } from "preact";
import {
  FormField,
  FormInput,
  FormSelect,
  FormTextarea,
  FormToggle,
  KeyValueEditor,
} from "../components/FormComponents";
import { HotkeyRecorder } from "../components/wizard/HotkeyRecorder";
import { SettingsField, WidgetDefinition, visibleFields } from "./types";

/**
 * Renders a widget's settings from its schema.
 *
 * This replaces the chain of `widget.type === "..." && h(...)` blocks the
 * wizard used to carry. That chain was where new widgets got forgotten — the
 * timer had no branch, so its config step rendered an empty panel with a Next
 * button, and nothing about the code made that omission visible.
 *
 * Now a widget with no fields has no config step at all (the wizard drops it),
 * and a widget with fields cannot have them silently missing.
 */
export function SettingsForm({
  definition,
  settings,
  errors,
  onChange,
}: {
  definition: WidgetDefinition;
  settings: Record<string, any>;
  /** Validation messages by field key, shown once the user has tried to advance. */
  errors?: Record<string, string>;
  onChange: (key: string, value: any) => void;
}) {
  const fields = visibleFields(definition, settings);

  if (fields.length === 0) {
    return h(
      "p",
      { class: "wizard-step-desc" },
      "This widget has nothing to configure.",
    );
  }

  return h(
    Fragment,
    null,
    fields.map((field) =>
      h(
        FormField,
        {
          key: field.key,
          label: field.label + (field.required ? " *" : ""),
          hint: field.hint,
          class: errors?.[field.key] ? "form-field-invalid" : undefined,
        },
        renderControl(field, settings, onChange),
        errors?.[field.key]
          ? h("p", { class: "form-error" }, errors[field.key])
          : null,
      ),
    ),
  );
}

function renderControl(
  field: SettingsField,
  settings: Record<string, any>,
  onChange: (key: string, value: any) => void,
): VNode<any> {
  const value = settings[field.key];
  const set = (next: any) => onChange(field.key, next);

  switch (field.kind) {
    case "text":
    case "url":
    case "password":
      return h(FormInput, {
        value: value ?? "",
        type: field.kind === "password" ? "password" : "text",
        placeholder: field.placeholder,
        onInput: set,
      });

    case "textarea":
      return h(FormTextarea, {
        value: value ?? "",
        placeholder: field.placeholder,
        rows: field.rows,
        onInput: set,
      });

    case "number":
      return h(FormInput, {
        value: String(value ?? field.default),
        type: "number",
        // Empty is allowed through as the default rather than NaN: a user
        // clearing the box to retype should not see the widget jump to 0.
        onInput: (raw) => set(clampNumber(raw, field.min, field.max, field.default)),
      });

    case "interval":
      return h(FormInput, {
        value: String(value ?? field.default),
        type: "number",
        onInput: (raw) =>
          set(clampNumber(raw, field.min ?? 250, undefined, field.default)),
      });

    case "select":
      return h(FormSelect, {
        value: String(value ?? field.default),
        options: field.options,
        onChange: set,
      });

    case "toggle":
      return h(FormToggle, {
        checked: Boolean(value ?? field.default),
        onChange: set,
      });

    case "hotkey":
      return h(HotkeyRecorder, {
        currentKeys: value ?? "",
        onChange: set,
      });

    case "keyvalue":
      return h(KeyValueEditor, {
        value: value ?? "",
        placeholder: field.placeholder,
        onChange: set,
      });

    case "custom":
      return field.render({ value, settings, onChange: set });
  }
}

function clampNumber(
  raw: string,
  min: number | undefined,
  max: number | undefined,
  fallback: number,
): number {
  const parsed = Number(raw);
  if (raw === "" || Number.isNaN(parsed)) return fallback;
  let result = parsed;
  if (min !== undefined) result = Math.max(min, result);
  if (max !== undefined) result = Math.min(max, result);
  return result;
}
