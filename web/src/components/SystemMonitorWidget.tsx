import { h } from "preact";
import { useState, useEffect } from "preact/hooks";
import { SystemStats } from "../lib/types";

export function SystemMonitorWidget({ settings }: { settings: Record<string, any> }) {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const variant = (settings.variant || "compact") as string;

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const res = await fetch("/api/system-stats");
        const data = await res.json();
        if (active && data.data) setStats(data.data);
      } catch (e) {}
    };
    load();
    const interval = setInterval(load, settings.refreshInterval || 2000);
    return () => { active = false; clearInterval(interval); };
  }, []);

  if (!stats) return h("div", { class: "sysmon-loading" }, "Loading...");

  function fmtBytes(b: number): string {
    if (b >= 1073741824) return (b / 1073741824).toFixed(1) + " GB";
    if (b >= 1048576) return (b / 1048576).toFixed(0) + " MB";
    return (b / 1024).toFixed(0) + " KB";
  }

  function fmtUptime(s: number): string {
    const d = Math.floor(s / 86400);
    const hh = Math.floor((s % 86400) / 3600);
    const m = Math.floor((s % 3600) / 60);
    if (d > 0) return `${d}d ${hh}h`;
    if (hh > 0) return `${hh}h ${m}m`;
    return `${m}m`;
  }

  const cpuColor = stats.cpu_usage < 50 ? "#4caf50" : stats.cpu_usage < 80 ? "#ff9800" : "#f44336";
  const memColor = stats.memory_usage < 60 ? "#2196f3" : stats.memory_usage < 85 ? "#ff9800" : "#f44336";

  if (variant === "minimal") {
    return h("div", { class: "sysmon-variant minimal" },
      h("div", { class: "sysmon-big", style: { color: cpuColor } }, `${stats.cpu_usage.toFixed(0)}%`),
      h("div", { class: "sysmon-big", style: { color: memColor } }, `${stats.memory_usage.toFixed(0)}%`),
    );
  }

  if (variant === "compact") {
    return h("div", { class: "sysmon-variant compact" },
      h("div", { class: "sysmon-cpu-model" }, stats.cpu_model.substring(0, 30)),
      h("div", { class: "sysmon-bar-group" },
        h("div", { class: "sysmon-bar-header" }, h("span", { class: "sysmon-bar-label" }, "CPU"), h("span", { class: "sysmon-bar-value", style: { color: cpuColor } }, `${stats.cpu_usage.toFixed(1)}%`)),
        h("div", { class: "sysmon-bar-track" }, h("div", { class: "sysmon-bar-fill", style: { width: `${stats.cpu_usage}%`, background: cpuColor } })),
      ),
      h("div", { class: "sysmon-bar-group" },
        h("div", { class: "sysmon-bar-header" }, h("span", { class: "sysmon-bar-label" }, "RAM"), h("span", { class: "sysmon-bar-value", style: { color: memColor } }, `${stats.memory_usage.toFixed(1)}%`)),
        h("div", { class: "sysmon-bar-track" }, h("div", { class: "sysmon-bar-fill", style: { width: `${stats.memory_usage}%`, background: memColor } })),
      ),
      h("div", { class: "sysmon-load-row" },
        h("span", null, `Load: ${stats.load_avg[0].toFixed(2)}`),
        h("span", null, `Up: ${fmtUptime(stats.uptime)}`),
      ),
    );
  }

  const swapPct = stats.swap_total > 0 ? (stats.swap_used / stats.swap_total) * 100 : 0;
  const swapColor = swapPct < 50 ? "#9c27b0" : swapPct < 80 ? "#ff9800" : "#f44336";

  return h("div", { class: "sysmon-variant detailed" },
    h("div", { class: "sysmon-cpu-model" }, stats.cpu_model),
    h("div", { class: "sysmon-bar-group" },
      h("div", { class: "sysmon-bar-header" }, h("span", { class: "sysmon-bar-label" }, "CPU"), h("span", { class: "sysmon-bar-value", style: { color: cpuColor } }, `${stats.cpu_usage.toFixed(1)}%`)),
      h("div", { class: "sysmon-bar-track" }, h("div", { class: "sysmon-bar-fill", style: { width: `${stats.cpu_usage}%`, background: cpuColor } })),
    ),
    h("div", { class: "sysmon-bar-group" },
      h("div", { class: "sysmon-bar-header" }, h("span", { class: "sysmon-bar-label" }, "Memory"), h("span", { class: "sysmon-bar-value", style: { color: memColor } }, `${stats.memory_usage.toFixed(1)}%`)),
      h("div", { class: "sysmon-bar-track" }, h("div", { class: "sysmon-bar-fill", style: { width: `${stats.memory_usage}%`, background: memColor } })),
      h("div", { class: "sysmon-bar-detail" }, `${fmtBytes(stats.memory_used)} / ${fmtBytes(stats.memory_total)}`),
    ),
    stats.swap_total > 0 && h("div", { class: "sysmon-bar-group" },
      h("div", { class: "sysmon-bar-header" }, h("span", { class: "sysmon-bar-label" }, "Swap"), h("span", { class: "sysmon-bar-value", style: { color: swapColor } }, `${swapPct.toFixed(1)}%`)),
      h("div", { class: "sysmon-bar-track" }, h("div", { class: "sysmon-bar-fill", style: { width: `${swapPct}%`, background: swapColor } })),
      h("div", { class: "sysmon-bar-detail" }, `${fmtBytes(stats.swap_used)} / ${fmtBytes(stats.swap_total)}`),
    ),
    h("div", { class: "sysmon-info-grid" },
      h("div", { class: "sysmon-info-item" }, h("div", { class: "sysmon-info-label" }, "Cores"), h("div", { class: "sysmon-info-value" }, String(stats.cpu_cores))),
      h("div", { class: "sysmon-info-item" }, h("div", { class: "sysmon-info-label" }, "Load"), h("div", { class: "sysmon-info-value" }, `${stats.load_avg[0].toFixed(2)} / ${stats.load_avg[1].toFixed(2)} / ${stats.load_avg[2].toFixed(2)}`)),
      h("div", { class: "sysmon-info-item" }, h("div", { class: "sysmon-info-label" }, "Uptime"), h("div", { class: "sysmon-info-value" }, fmtUptime(stats.uptime))),
      h("div", { class: "sysmon-info-item" }, h("div", { class: "sysmon-info-label" }, "Processes"), h("div", { class: "sysmon-info-value" }, `${stats.process_count} / ${stats.thread_count}`)),
    ),
  );
}
