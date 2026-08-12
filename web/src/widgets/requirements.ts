/**
 * Checking a widget's requirements against what the host actually offers.
 *
 * The dashboard shipped widgets that could not possibly work — an OBS widget
 * with the OBS plugin disabled renders "not connected to OBS", which reads as
 * "start OBS" when the real answer is "enable the plugin". Requirements make
 * that difference visible before the widget is ever added, and again in the
 * wizard, and again on the widget itself if the situation changes later.
 */

import { PluginStatus } from "../lib/types";
import { Requirement, WidgetDefinition } from "./types";

/** What the host currently provides. */
export interface HostStatus {
  plugins: PluginStatus[];
  /** False while the first fetch is in flight, so callers can avoid flashing a false negative. */
  loaded: boolean;
}

export const UNKNOWN_HOST_STATUS: HostStatus = { plugins: [], loaded: false };

export interface RequirementResult {
  requirement: Requirement;
  met: boolean;
  /** What the user should do about it, when it is not met. */
  remedy: string;
  /** What is wrong, in a few words. */
  status: string;
}

export interface RequirementReport {
  results: RequirementResult[];
  /** True when every requirement is met, or the widget declares none. */
  satisfied: boolean;
  /** Unmet requirements only. */
  unmet: RequirementResult[];
}

/**
 * Evaluate a widget's requirements.
 *
 * While the host status is still loading, everything is reported as met. A
 * requirement panel that flashes red for half a second on every open trains
 * people to ignore it.
 */
export function checkRequirements(
  definition: WidgetDefinition,
  host: HostStatus,
): RequirementReport {
  const requirements = definition.requires ?? [];

  const results = requirements.map((requirement) =>
    evaluate(requirement, host),
  );
  const unmet = results.filter((result) => !result.met);

  return { results, unmet, satisfied: unmet.length === 0 };
}

function evaluate(requirement: Requirement, host: HostStatus): RequirementResult {
  const plugin = host.plugins.find((p) => p.name === requirement.plugin);

  if (!host.loaded) {
    return {
      requirement,
      met: true,
      status: "Checking...",
      remedy: "",
    };
  }

  if (!plugin) {
    return {
      requirement,
      met: false,
      status: "Not installed",
      remedy: `Install the "${requirement.plugin}" plugin from the Plugins page.`,
    };
  }

  if (!plugin.enabled) {
    return {
      requirement,
      met: false,
      status: "Disabled",
      remedy: `Enable "${requirement.plugin}" on the Plugins page.`,
    };
  }

  if (!plugin.loaded) {
    return {
      requirement,
      met: false,
      status: "Not loaded",
      remedy: `"${requirement.plugin}" is enabled but failed to load. Check the server log, then use Refresh on the Plugins page.`,
    };
  }

  return { requirement, met: true, status: "Ready", remedy: "" };
}

/** One-line summary for a library tile or a widget placeholder. */
export function summarizeUnmet(report: RequirementReport): string {
  if (report.satisfied) return "";
  const names = report.unmet.map((r) => r.requirement.plugin);
  return names.length === 1
    ? `Needs the ${names[0]} plugin`
    : `Needs plugins: ${names.join(", ")}`;
}
