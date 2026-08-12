import { h } from "preact";
import { useState } from "preact/hooks";
import { setToken } from "../lib/auth";

/**
 * Shown when the dashboard could not obtain an API token on its own.
 *
 * In practice this means the page is open on a phone or another machine: the
 * loopback bootstrap only answers callers on the daemon's own host, so a
 * remote visitor has to paste the token printed at startup (or scan the QR
 * code, which carries it in the URL).
 */
export function TokenGate({ onAuthenticated }: { onAuthenticated: () => void }) {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  const submit = async (e: Event) => {
    e.preventDefault();
    const candidate = value.trim();
    if (!candidate) return;

    setChecking(true);
    setError(null);
    setToken(candidate);

    try {
      const res = await fetch("/api/devices");
      if (res.status === 401) {
        setToken(null);
        setError("That token was rejected. Check it against the server output.");
      } else if (!res.ok) {
        setToken(null);
        setError(`The server answered ${res.status}. Is it still running?`);
      } else {
        onAuthenticated();
        return;
      }
    } catch {
      setToken(null);
      setError("Could not reach the server.");
    } finally {
      setChecking(false);
    }
  };

  return h(
    "div",
    { class: "token-gate" },
    h(
      "form",
      { class: "token-gate-card", onSubmit: submit },
      h("h1", null, "API token required"),
      h(
        "p",
        null,
        "This dashboard is not running on the machine hosting sd-core, so it " +
          "cannot fetch the token itself. Paste the token printed in the " +
          "server's startup output, or scan the QR code from the local " +
          "dashboard instead.",
      ),
      h("input", {
        class: "token-gate-input",
        type: "password",
        value,
        autocomplete: "off",
        placeholder: "API token",
        onInput: (e: Event) => setValue((e.target as HTMLInputElement).value),
      }),
      error && h("p", { class: "token-gate-error" }, error),
      h(
        "button",
        { class: "token-gate-submit", type: "submit", disabled: checking },
        checking ? "Checking..." : "Connect",
      ),
    ),
  );
}
