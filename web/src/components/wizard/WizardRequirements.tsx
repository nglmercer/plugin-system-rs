import { h } from "preact";
import { t } from "../../lib/i18n";
import { RequirementReport } from "../../widgets";

/**
 * The step that says what a widget needs before you configure it.
 *
 * Previously a widget whose plugin was missing looked configurable right up
 * until it was on the dashboard rendering "not connected to OBS" — which reads
 * as "start OBS" when the real problem is that the plugin is disabled. This
 * puts that distinction in front of the user first, with the specific action
 * that fixes it.
 *
 * It does not block. An unmet requirement is usually temporary (OBS not running
 * yet, plugin about to be enabled), and refusing to let someone lay out their
 * dashboard until the host is perfect would be worse than a warning.
 */
export function WizardRequirements({ report }: { report: RequirementReport }) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, t("widget.wizard.requirements")),
    h(
      "p",
      { class: "wizard-step-desc" },
      report.satisfied
        ? t("widget.wizard.requirementsOk")
        : t("widget.wizard.requirementsMissing"),
    ),
    h(
      "div",
      { class: "requirement-list" },
      report.results.map((result) =>
        h(
          "div",
          {
            class: `requirement-row ${result.met ? "met" : "unmet"}`,
            key: result.requirement.plugin,
          },
          h("div", { class: "requirement-mark" }, result.met ? "✓" : "!"),
          h(
            "div",
            { class: "requirement-body" },
            h(
              "div",
              { class: "requirement-title" },
              h("span", { class: "requirement-name" }, result.requirement.plugin),
              h("span", { class: "requirement-status" }, result.status),
            ),
            h("div", { class: "requirement-reason" }, result.requirement.reason),
            result.remedy
              ? h("div", { class: "requirement-remedy" }, result.remedy)
              : null,
          ),
        ),
      ),
    ),
  );
}
