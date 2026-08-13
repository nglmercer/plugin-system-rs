/**
 * Registers every built-in widget, and re-exports the widget API.
 *
 * This file is the only list of shipped widgets. Adding one means writing a
 * definition module and adding it to the array below — nothing else in the app
 * needs to learn about it.
 */

import { registerWidgets } from "./registry";

import { clockWidget } from "./definitions/clock";
import { fetchWidget } from "./definitions/fetch";
import { obsControlWidget } from "./definitions/obsControl";
import { obsInputsWidget } from "./definitions/obsInputs";
import { obsMediaWidget } from "./definitions/obsMedia";
import { obsScenesWidget } from "./definitions/obsScenes";
import { obsStudioWidget } from "./definitions/obsStudio";
import { openUrlWidget } from "./definitions/openUrl";
import { pluginDataWidget } from "./definitions/pluginData";
import { sendHotkeyWidget } from "./definitions/sendHotkey";
import { systemMonitorWidget } from "./definitions/systemMonitor";
import { timerWidget } from "./definitions/timer";
import { typeTextWidget } from "./definitions/typeText";
import { volumeAppsWidget } from "./definitions/volumeApps";
import { volumeMasterWidget } from "./definitions/volumeMaster";

/** Order here is the order the widget library shows within a category. */
export const BUILTIN_WIDGETS = [
  systemMonitorWidget,
  clockWidget,
  timerWidget,
  sendHotkeyWidget,
  typeTextWidget,
  openUrlWidget,
  volumeMasterWidget,
  volumeAppsWidget,
  obsControlWidget,
  obsScenesWidget,
  obsInputsWidget,
  obsStudioWidget,
  obsMediaWidget,
  fetchWidget,
  pluginDataWidget,
];

let registered = false;

/** Idempotent: safe to call from both app start-up and a test's setup. */
export function registerBuiltinWidgets(): void {
  if (registered) return;
  registered = true;
  registerWidgets(BUILTIN_WIDGETS);
}

export * from "./types";
export * from "./registry";
export * from "./requirements";
export { useHostStatus, refreshHostStatus, invalidateHostStatus, hostStatusSnapshot } from "./useHostStatus";
export { SettingsForm } from "./SettingsForm";
export { wizardStepIds } from "./wizardSteps";
export type { WizardStepId } from "./wizardSteps";
export { installExternalWidgetApi } from "./external";
