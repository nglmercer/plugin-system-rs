import { h, VNode } from "preact";
import { useMemo, useState } from "preact/hooks";
import { t, tOr } from "../lib/i18n";
import { WidgetConfig } from "../lib/types";
import {
  checkRequirements,
  defaultSettings,
  getWidget,
  useHostStatus,
  validateSettings,
  wizardStepIds,
  WizardStepId,
} from "../widgets";
import { WizardConfig } from "./WidgetWizard/WizardConfig";
import { WizardConfirm } from "./WidgetWizard/WizardConfirm";
import { WizardGeneral } from "./WidgetWizard/WizardGeneral";
import { WizardRequirements } from "./WidgetWizard/WizardRequirements";
import { WizardStyle } from "./WidgetWizard/WizardStyle";

interface WidgetWizardProps {
  widget: WidgetConfig;
  columns: number;
  rows: number;
  onSave: (
    id: string,
    updates: {
      title?: string;
      colSpan?: number;
      rowSpan?: number;
      settings?: Record<string, any>;
    },
  ) => void;
  onRemove: () => void;
  onClose: () => void;
}

interface Step {
  id: WizardStepId;
  label: string;
  render: () => VNode<any>;
  /** Message shown when the user tries to leave a step that is not finished. */
  blockedBy?: () => string | null;
}

/**
 * The widget editor.
 *
 * Steps are assembled from the widget's definition rather than fixed, because a
 * fixed list produces empty steps. Every widget used to get a Config step and a
 * Style step whether or not it had anything to put in them — the clock's config
 * step was a heading and a Next button.
 *
 * Now: Requirements appears only when the widget needs something from the host,
 * Config only when it has settings, Style only when it has more than one
 * variant. A widget with none of those is Title → Apply, two steps.
 */
export function WidgetWizard({
  widget,
  columns,
  rows,
  onSave,
  onRemove,
  onClose,
}: WidgetWizardProps) {
  const definition = getWidget(widget.type);
  const host = useHostStatus();

  const [step, setStep] = useState(0);
  const [title, setTitle] = useState(widget.title);
  const [colSpan, setColSpan] = useState(widget.colSpan);
  const [rowSpan, setRowSpan] = useState(widget.rowSpan);
  const [settings, setSettings] = useState<Record<string, any>>(() => ({
    // A layout saved before a field existed has no value for it, and a widget
    // silently missing a setting reads as a broken widget. Backfilling from the
    // schema means adding a field is not a migration.
    ...(definition ? defaultSettings(definition) : {}),
    ...widget.settings,
  }));
  const [showErrors, setShowErrors] = useState(false);

  const variant = String(
    settings.variant ?? definition?.defaultVariant ?? definition?.variants?.[0]?.value ?? "",
  );

  const requirements = useMemo(
    () => (definition ? checkRequirements(definition, host) : { results: [], unmet: [], satisfied: true }),
    [definition, host],
  );

  const errors = useMemo(
    () => (definition ? validateSettings(definition, settings) : {}),
    [definition, settings],
  );

  if (!definition) {
    return h(
      Overlay,
      { onClose },
      h(
        "div",
        { class: "wizard-body" },
        h(
          "p",
          { class: "wizard-step-desc" },
          t("widget.unknownType", { type: widget.type }),
        ),
      ),
    );
  }

  function updateSetting(key: string, value: any) {
    setSettings((previous) => ({ ...previous, [key]: value }));
  }

  const renderers: Record<WizardStepId, () => Step> = {
    requirements: () => ({
      id: "requirements",
      label: t("widget.wizard.requirements"),
      render: () => h(WizardRequirements, { report: requirements }),
    }),
    general: () => ({
      id: "general",
      label: t("widget.wizard.general"),
      render: () =>
        h(WizardGeneral, {
          values: { title, colSpan, rowSpan },
          columns,
          rows,
          onChange: (values) => {
            if (values.title !== undefined) setTitle(values.title);
            if (values.colSpan !== undefined) setColSpan(values.colSpan);
            if (values.rowSpan !== undefined) setRowSpan(values.rowSpan);
          },
        }),
      blockedBy: () => (title.trim() ? null : t("widget.wizard.titleRequired")),
    }),
    config: () => ({
      id: "config",
      label: t("widget.wizard.config"),
      render: () =>
        h(WizardConfig, {
          definition,
          settings,
          errors: showErrors ? errors : {},
          onChange: updateSetting,
        }),
      // Config is the step with real requirements: a Fetch widget with no URL
      // is not configured, and letting it through only moves the failure onto
      // the dashboard where it is harder to explain.
      blockedBy: () => {
        const count = Object.keys(errors).length;
        return count === 0 ? null : t("widget.wizard.fixFields", { count });
      },
    }),
    style: () => ({
      id: "style",
      label: t("widget.wizard.style"),
      render: () =>
        h(WizardStyle, {
          definition,
          variant,
          onChange: (value) => updateSetting("variant", value),
        }),
    }),
    apply: () => ({
      id: "apply",
      label: t("widget.wizard.apply"),
      render: () =>
        h(WizardConfirm, {
          definition,
          widget,
          title,
          colSpan,
          rowSpan,
          settings,
          variant,
          requirements,
          onRemove,
        }),
    }),
  };

  const steps: Step[] = wizardStepIds(definition).map((id) => renderers[id]());

  const current = Math.min(step, steps.length - 1);
  const isLast = current === steps.length - 1;
  const blocker = steps[current].blockedBy?.() ?? null;

  function goTo(target: number) {
    // Moving backwards is always allowed; moving forwards past an unfinished
    // step is not, and the reason is shown rather than the button just going
    // dead.
    if (target <= current) {
      setStep(target);
      return;
    }
    if (blocker) {
      setShowErrors(true);
      return;
    }
    setShowErrors(false);
    setStep(target);
  }

  function handleApply() {
    if (Object.keys(errors).length > 0) {
      setShowErrors(true);
      // Land on the step that has the problem instead of failing silently.
      const configStep = steps.findIndex((s) => s.id === "config");
      if (configStep >= 0) setStep(configStep);
      return;
    }
    onSave(widget.id, {
      title,
      colSpan,
      rowSpan,
      settings: { ...settings, variant },
    });
  }

  return h(
    Overlay,
    { onClose },
    h(
      "div",
      { class: "wizard-header" },
      h(
        "div",
        { class: "wizard-title" },
        h("span", { class: "wizard-title-icon" }, h(definition.icon, null)),
        h("span", null, tOr(`widget.types.${definition.type}`, definition.label)),
      ),
      h(
        "button",
        { class: "wizard-close", type: "button", onClick: onClose, "aria-label": t("common.close") },
        "✕",
      ),
    ),
    h(
      "div",
      { class: "wizard-steps" },
      steps.map((entry, index) =>
        h(
          "button",
          {
            class: `wizard-step-indicator ${index === current ? "active" : index < current ? "done" : ""}`,
            key: entry.id,
            type: "button",
            onClick: () => goTo(index),
          },
          h("div", { class: "wizard-step-circle" }, index < current ? "✓" : String(index + 1)),
          h("div", { class: "wizard-step-label" }, entry.label),
        ),
      ),
    ),
    h("div", { class: "wizard-body" }, steps[current].render()),
    h(
      "div",
      { class: "wizard-footer" },
      current > 0
        ? h(
            "button",
            { class: "wizard-btn back", type: "button", onClick: () => goTo(current - 1) },
            t("widget.wizard.back"),
          )
        : null,
      h(
        "div",
        { class: "wizard-footer-spacer" },
        showErrors && blocker ? h("span", { class: "wizard-blocker" }, blocker) : null,
      ),
      isLast
        ? h(
            "button",
            { class: "wizard-btn apply", type: "button", onClick: handleApply },
            t("widget.wizard.save"),
          )
        : h(
            "button",
            {
              class: `wizard-btn next ${blocker ? "blocked" : ""}`,
              type: "button",
              onClick: () => goTo(current + 1),
            },
            t("widget.wizard.next"),
          ),
    ),
  );
}

function Overlay({ onClose, children }: { onClose: () => void; children?: any }) {
  return h(
    "div",
    { class: "wizard-overlay", onClick: onClose },
    h(
      "div",
      { class: "wizard-modal", onClick: (e: Event) => e.stopPropagation() },
      children,
    ),
  );
}
