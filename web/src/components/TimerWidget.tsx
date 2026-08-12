import { h } from "preact";
import { useState } from "preact/hooks";
import { fetchTimers, startTimer, stopTimer, TimerSnapshot } from "../lib/api";
import { usePolling } from "../lib/usePolling";

/**
 * Countdown timers, backed by the `timer` plugin.
 *
 * The plugin holds the deadline and derives the remaining time from the WASI
 * clock on every read, so this widget only polls and renders — it never runs a
 * countdown of its own. That matters: a local `setInterval` would drift away
 * from the plugin's answer and disagree with any other client watching the
 * same timer.
 */
export function TimerWidget({ settings }: { settings: Record<string, any> }) {
  const variant = (settings.variant || "compact") as string;
  const timerName = (settings.timerName || "timer") as string;
  const seconds = Number(settings.seconds) || 300;
  const refreshInterval = Number(settings.refreshInterval) || 1000;

  const [timers, setTimers] = useState<TimerSnapshot[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  usePolling(async () => {
    try {
      setTimers(await fetchTimers());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      throw e;
    }
  }, refreshInterval);

  const timer = timers.find((t) => t.name === timerName) ?? null;

  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
      setTimers(await fetchTimers());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  if (error && timers.length === 0) {
    return h("div", { class: "timer-widget error" }, error);
  }

  if (variant === "minimal") {
    return h(
      "div",
      { class: "timer-widget minimal" },
      h(
        "div",
        { class: `timer-time ${timer?.expired ? "expired" : ""}` },
        timer ? formatDuration(timer.remaining) : "--:--",
      ),
    );
  }

  if (variant === "detailed") {
    return h(
      "div",
      { class: "timer-widget detailed" },
      timers.length === 0
        ? h("div", { class: "timer-empty" }, "No timers running")
        : timers.map((t) =>
            h(
              "div",
              { class: "timer-row", key: t.name },
              h("span", { class: "timer-row-name" }, t.name),
              h("div", { class: "timer-bar" }, h("div", {
                class: `timer-bar-fill ${t.expired ? "expired" : ""}`,
                style: { width: `${progressPercent(t)}%` },
              })),
              h(
                "span",
                { class: `timer-row-time ${t.expired ? "expired" : ""}` },
                formatDuration(t.remaining),
              ),
              h(
                "button",
                {
                  class: "timer-btn",
                  disabled: busy,
                  onClick: () => run(() => stopTimer(t.name)),
                },
                "Stop",
              ),
            ),
          ),
      error ? h("div", { class: "timer-error" }, error) : null,
    );
  }

  return h(
    "div",
    { class: "timer-widget compact" },
    h("div", { class: "timer-name" }, timerName),
    h(
      "div",
      { class: `timer-time ${timer?.expired ? "expired" : ""}` },
      timer ? formatDuration(timer.remaining) : formatDuration(seconds),
    ),
    h(
      "div",
      { class: "timer-controls" },
      h(
        "button",
        {
          class: "timer-btn",
          disabled: busy,
          onClick: () => run(() => startTimer(timerName, seconds)),
        },
        timer && !timer.expired ? "Restart" : "Start",
      ),
      timer
        ? h(
            "button",
            {
              class: "timer-btn",
              disabled: busy,
              onClick: () => run(() => stopTimer(timerName)),
            },
            "Stop",
          )
        : null,
    ),
    error ? h("div", { class: "timer-error" }, error) : null,
  );
}

/** Seconds as `H:MM:SS`, or `M:SS` under an hour. */
function formatDuration(totalSeconds: number): string {
  const safe = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const secs = safe % 60;

  const pad = (n: number) => n.toString().padStart(2, "0");
  return hours > 0
    ? `${hours}:${pad(minutes)}:${pad(secs)}`
    : `${minutes}:${pad(secs)}`;
}

/** How much of the timer has elapsed, as a percentage. */
function progressPercent(timer: TimerSnapshot): number {
  if (timer.seconds <= 0) return 100;
  const elapsed = timer.seconds - timer.remaining;
  return Math.min(100, Math.max(0, (elapsed / timer.seconds) * 100));
}
