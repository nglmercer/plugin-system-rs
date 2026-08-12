import { h } from "preact";
import { t } from "../../lib/i18n";
import { SettingsForm, WidgetDefinition } from "../../widgets";

/**
 * The config step is now just the widget's schema, rendered.
 *
 * It used to be a chain of `widget.type === "..."` branches, which is where new
 * widgets were forgotten: a type with no branch got an empty panel and a Next
 * button. A widget with no settings no longer reaches this step at all — the
 * wizard drops it — so an empty config step is impossible rather than merely
 * unlikely.
 */
export function WizardConfig({
  definition,
  settings,
  errors,
  onChange,
}: {
  definition: WidgetDefinition;
  settings: Record<string, any>;
  errors: Record<string, string>;
  onChange: (key: string, value: any) => void;
}) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, t("widget.wizard.config")),
    h("p", { class: "wizard-step-desc" }, t("widget.wizard.configDesc")),
    h(SettingsForm, { definition, settings, errors, onChange }),
  );
}
