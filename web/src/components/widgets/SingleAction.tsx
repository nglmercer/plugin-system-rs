import { h } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";
import "./singleAction.css";

/**
 * The one-button widgets (Type Text, Open URL) used to be near-identical
 * copies: same state pair, same execute dance, same markup. The shared half
 * lives here; each widget keeps only what is actually its own.
 */

const RESULT_VISIBLE_MS = 3000;

/** Run an action, hold its result on screen briefly, never overlap runs. */
export function useAction(run: () => Promise<string>) {
  const [executing, setExecuting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);

  async function execute() {
    if (executing) return;
    setExecuting(true);
    setResult(null);
    try {
      setResult(await run());
    } catch {
      setResult("Error");
    }
    setExecuting(false);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setResult(null), RESULT_VISIBLE_MS);
  }

  return { executing, result, execute };
}

export interface SingleActionViewProps {
  detailed: boolean;
  /** The payload shown big (the text, the URL). */
  value: string;
  description?: string;
  executing: boolean;
  result: string | null;
  onExecute: () => void;
  buttonLabel: string;
}

export function SingleActionView({
  detailed,
  value,
  description,
  executing,
  result,
  onExecute,
  buttonLabel,
}: SingleActionViewProps) {
  return h(
    "div",
    { class: `action-single${detailed ? " detailed" : ""}` },
    h("div", { class: "action-single-value" }, value),
    detailed && description ? h("div", { class: "action-single-desc" }, description) : null,
    result ? h("div", { class: "action-result" }, result) : null,
    h(
      "button",
      {
        class: `action-single-btn${executing ? " executing" : ""}`,
        type: "button",
        onClick: onExecute,
        disabled: executing,
      },
      buttonLabel,
    ),
  );
}
