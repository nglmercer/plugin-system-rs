/**
 * Which wizard steps a widget gets.
 *
 * Pulled out of the wizard component so it is a plain function of the
 * definition, and therefore testable. The rule it encodes is the one the old
 * fixed four-step wizard got wrong: a step exists only when it has something to
 * show. Every widget used to get a Config step and a Style step regardless —
 * the clock's config step was a heading and a Next button, and the timer's was
 * empty because nobody had added its branch to the `if` chain.
 */

import { WidgetDefinition } from "./types";

export type WizardStepId = "requirements" | "general" | "config" | "style" | "apply";

export function wizardStepIds(definition: WidgetDefinition): WizardStepId[] {
  const steps: WizardStepId[] = [];

  // Only when the widget actually depends on the host for something.
  if ((definition.requires?.length ?? 0) > 0) steps.push("requirements");

  // Always: every widget has a title and a size.
  steps.push("general");

  // Only when there is at least one field to fill in.
  if ((definition.settings?.length ?? 0) > 0) steps.push("config");

  // Only when there is a choice to make. One variant is not a choice.
  if ((definition.variants?.length ?? 0) > 1) steps.push("style");

  // Always: the review and the delete button live here.
  steps.push("apply");

  return steps;
}
