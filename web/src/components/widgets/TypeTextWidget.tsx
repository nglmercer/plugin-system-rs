import { h } from "preact";
import { executeAction } from "../../lib/api";
import { SingleActionView, useAction } from "./SingleAction";

export function TypeTextWidget({ settings }: { settings: Record<string, any> }) {
  const { executing, result, execute } = useAction(() => executeAction("Type Text"));

  return h(SingleActionView, {
    detailed: settings.variant === "detailed",
    value: settings.text || "No text set",
    description: "Type text string",
    executing,
    result,
    onExecute: execute,
    buttonLabel: executing ? "Typing..." : "Type",
  });
}
