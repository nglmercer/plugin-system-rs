import { h } from "preact";
import { openUrl } from "../../lib/api";
import { SingleActionView, useAction } from "./SingleAction";

export function OpenUrlWidget({ settings }: { settings: Record<string, any> }) {
  const { executing, result, execute } = useAction(async () => {
    const url = settings.url;
    if (!url) return "No URL set";
    return openUrl(url);
  });

  return h(SingleActionView, {
    detailed: settings.variant === "detailed",
    value: settings.url || "No URL set",
    description: "Open URL in browser",
    executing,
    result,
    onExecute: execute,
    buttonLabel: executing ? "Opening..." : "Open",
  });
}
