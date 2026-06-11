import { h } from "preact";
import { useState, useEffect } from "preact/hooks";
import { executeAction } from "../lib/api";

export function QuickActionsWidget() {
  const [actions, setActions] = useState<string[]>([]);
  const [executing, setExecuting] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);

  useEffect(() => {
    fetch("/api/actions").then((r) => r.json()).then((d) => setActions(d.data || [])).catch(() => {});
  }, []);

  async function handleExecute(actionName: string) {
    setExecuting(actionName);
    setResult(null);
    try { setResult(await executeAction(actionName)); } catch { setResult("Error"); }
    setExecuting(null);
    setTimeout(() => setResult(null), 3000);
  }

  return h("div", { class: "quick-actions-widget" },
    result && h("div", { class: "action-result" }, result),
    actions.map((a, i) => {
      const name = a.split(" (")[0];
      const cat = a.match(/\((.+)\)/)?.[1] || "";
      return h("button", { class: `action-btn ${executing === a ? "executing" : ""}`, key: i, onClick: () => handleExecute(name), disabled: executing !== null },
        h("span", { class: "action-btn-name" }, name),
        h("span", { class: "action-btn-cat" }, cat),
      );
    }),
  );
}
