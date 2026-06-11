import { h } from "preact";
import { useState, useEffect } from "preact/hooks";

export function ClockWidget({ settings }: { settings: Record<string, any> }) {
  const [time, setTime] = useState(() => new Date());
  const variant = (settings.variant || "digital") as string;

  useEffect(() => { const i = setInterval(() => setTime(new Date()), 1000); return () => clearInterval(i); }, []);

  const hh = time.getHours().toString().padStart(2, "0");
  const mm = time.getMinutes().toString().padStart(2, "0");
  const ss = time.getSeconds().toString().padStart(2, "0");

  if (variant === "simple") return h("div", { class: "clock-variant simple" }, h("div", { class: "clock-time" }, `${hh}:${mm}`));
  if (variant === "detailed") return h("div", { class: "clock-variant detailed" },
    h("div", { class: "clock-time" }, `${hh}:${mm}:${ss}`),
    h("div", { class: "clock-date" }, time.toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric", year: "numeric" })),
  );
  return h("div", { class: "clock-variant digital" },
    h("div", { class: "clock-time" }, `${hh}:${mm}`),
    h("div", { class: "clock-seconds" }, ss),
    h("div", { class: "clock-date" }, time.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric", year: "numeric" })),
  );
}
