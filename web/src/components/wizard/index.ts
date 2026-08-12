/**
 * Wizard pieces.
 *
 * `FetchConfig` and `IntervalField` are gone: both were per-widget config built
 * by hand, and both are now expressed as `settings` fields on the widget
 * definitions that needed them. `HotkeyRecorder` survives because it is a real
 * custom control, rendered by the schema for the `hotkey` field kind.
 */
export { WidgetWizard } from "./WidgetWizard";
export { WizardGeneral } from "./WizardGeneral";
export { WizardConfig } from "./WizardConfig";
export { WizardRequirements } from "./WizardRequirements";
export { WizardStyle } from "./WizardStyle";
export { WizardConfirm } from "./WizardConfirm";
export { HotkeyRecorder } from "./HotkeyRecorder";
