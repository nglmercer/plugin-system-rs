import { h } from "preact";
import { useState } from "preact/hooks";
import { sendHotkeyCombo, executeAction } from "../lib/api";
import { Icons } from "./Icons";

export function SendHotkeyWidget({ settings }: { settings: Record<string, any> }) {
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;

  async function handleExecute() {
    setExecuting(true); setResult(null);
    try { setResult(settings.keys ? await sendHotkeyCombo(settings.keys) : await executeAction("Send Hotkey")); } catch { setResult("Error"); }
    setExecuting(false);
    setTimeout(() => setResult(null), 3000);
  }

  const keys = (settings.keys || "").split("+").map((k: string) => k.trim()).filter(Boolean);

  return h("div", {
    class: `hotkey-widget ${variant}`,
    onClick: handleExecute,
    style: executing ? { opacity: 0.6 } : undefined,
  },
    h("div", { class: "hotkey-center" },
      h("div", { class: "hotkey-icon" }, h(Icons.keyboard, null)),
      h("div", { class: "hotkey-keys" },
        keys.length > 0
          ? keys.map((k: string, i: number) => h("span", { key: i, class: "hotkey-key" }, k))
          : h("span", { class: "hotkey-key empty" }, "Not set")
      ),
      result && h("div", { class: "hotkey-result" }, result),
    ),
  );
}
