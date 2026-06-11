import { h } from "preact";
import { useState } from "preact/hooks";
import { executeAction } from "../lib/api";

export function OpenUrlWidget({ settings }: { settings: Record<string, any> }) {
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;

  async function handleExecute() {
    setExecuting(true); setResult(null);
    try { setResult(await executeAction("Open URL")); } catch { setResult("Error"); }
    setExecuting(false);
    setTimeout(() => setResult(null), 3000);
  }

  return h("div", { class: `action-single-widget ${variant === "detailed" ? "detailed" : ""}` },
    h("div", { class: "action-single-url" }, settings.url || "No URL set"),
    variant === "detailed" && h("div", { class: "action-single-desc" }, "Open URL in browser"),
    result && h("div", { class: "action-result" }, result),
    h("button", { class: `action-single-btn ${executing ? "executing" : ""}`, onClick: handleExecute, disabled: executing },
      executing ? "Opening..." : "Open"),
  );
}
