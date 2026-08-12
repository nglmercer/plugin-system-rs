import { h } from "preact";
import { useState } from "preact/hooks";
import { SystemStats } from "../../lib/types";
import { fetchSystemStats } from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import { formatBytes, formatUptime } from "../../lib/format";
import { levelColor, refreshMs } from "./widgetUtils";
import { WidgetLoading } from "./widgetParts";
import "./sysmon.css";

export function SystemMonitorWidget({ settings }: { settings: Record<string, any> }) {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const variant = (settings.variant || "compact") as string;

  usePolling(async () => {
    const data = await fetchSystemStats();
    if (data) setStats(data);
  }, refreshMs(settings));

  if (!stats) return h(WidgetLoading, null);

  const cpuColor = levelColor(stats.cpu_usage);
  const memColor = stats.memory_usage < 60 ? "var(--sd-info)" : levelColor(stats.memory_usage);

  if (variant === "minimal") {
    return h("div", { class: "sysmon-variant minimal" },
      h("div", { class: "sysmon-big", style: { color: cpuColor } }, `${stats.cpu_usage.toFixed(0)}%`),
      h("div", { class: "sysmon-big", style: { color: memColor } }, `${stats.memory_usage.toFixed(0)}%`),
    );
  }

  if (variant === "compact") {
    return h("div", { class: "sysmon-variant compact" },
      h("div", { class: "sysmon-cpu-model" }, stats.cpu_model.substring(0, 30)),
      h(Bar, { label: "CPU", pct: stats.cpu_usage, color: cpuColor }),
      h(Bar, { label: "RAM", pct: stats.memory_usage, color: memColor }),
      h("div", { class: "sysmon-load-row" },
        h("span", null, `Load: ${stats.load_avg[0].toFixed(2)}`),
        h("span", null, `Up: ${formatUptime(stats.uptime)}`),
      ),
    );
  }

  const swapPct = stats.swap_total > 0 ? (stats.swap_used / stats.swap_total) * 100 : 0;
  const swapColor = swapPct < 50 ? "var(--sd-accent-alt)" : levelColor(swapPct);

  return h("div", { class: "sysmon-variant detailed" },
    h("div", { class: "sysmon-cpu-model" }, stats.cpu_model),
    h(Bar, { label: "CPU", pct: stats.cpu_usage, color: cpuColor }),
    h(Bar, {
      label: "Memory",
      pct: stats.memory_usage,
      color: memColor,
      detail: `${formatBytes(stats.memory_used)} / ${formatBytes(stats.memory_total)}`,
    }),
    stats.swap_total > 0 && h(Bar, {
      label: "Swap",
      pct: swapPct,
      color: swapColor,
      detail: `${formatBytes(stats.swap_used)} / ${formatBytes(stats.swap_total)}`,
    }),
    h("div", { class: "stat-grid" },
      h(InfoCell, { label: "Cores", value: String(stats.cpu_cores) }),
      h(InfoCell, { label: "Load", value: `${stats.load_avg[0].toFixed(2)} / ${stats.load_avg[1].toFixed(2)} / ${stats.load_avg[2].toFixed(2)}` }),
      h(InfoCell, { label: "Uptime", value: formatUptime(stats.uptime) }),
      h(InfoCell, { label: "Processes", value: `${stats.process_count} / ${stats.thread_count}` }),
    ),
  );
}

function Bar({ label, pct, color, detail }: { label: string; pct: number; color: string; detail?: string }) {
  return h("div", { class: "sysmon-bar-group" },
    h("div", { class: "sysmon-bar-header" },
      h("span", { class: "sysmon-bar-label" }, label),
      h("span", { class: "sysmon-bar-value", style: { color } }, `${pct.toFixed(1)}%`),
    ),
    h("div", { class: "sysmon-bar-track" },
      h("div", { class: "sysmon-bar-fill", style: { width: `${pct}%`, background: color } }),
    ),
    detail ? h("div", { class: "sysmon-bar-detail" }, detail) : null,
  );
}

function InfoCell({ label, value }: { label: string; value: string }) {
  return h("div", { class: "stat-cell" },
    h("div", { class: "stat-label" }, label),
    h("div", { class: "stat-value" }, value),
  );
}
