import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";

interface VolumeState {
  master_volume: number;
  muted: boolean;
  default_device_name: string;
  platform_supported: boolean;
  per_app_supported: boolean;
}

interface AppVolume {
  name: string;
  volume: number;
  muted: boolean;
  pid: number | null;
}

export function VolumeWidget({ settings }: { settings: Record<string, any> }) {
  const [state, setState] = useState<VolumeState | null>(null);
  const [apps, setApps] = useState<AppVolume[]>([]);
  const [showApps, setShowApps] = useState(false);
  const variant = (settings.variant || "compact") as string;

  const loadVolume = useCallback(async () => {
    try {
      const res = await fetch("/api/volume");
      const data = await res.json();
      if (data.success && data.data) {
        const d = data.data;
        setState({
          master_volume: d.state?.master_volume ?? 0,
          muted: d.state?.muted ?? false,
          default_device_name: d.state?.default_device_name ?? "",
          platform_supported: d.state?.platform_supported ?? false,
          per_app_supported: d.state?.per_app_supported ?? false,
        });
        if (d.apps) setApps(d.apps);
      }
    } catch (e) {}
  }, []);

  useEffect(() => {
    loadVolume();
    const interval = setInterval(() => {
      loadVolume();
    }, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, []);

  const setVolume = async (vol: number) => {
    try {
      await fetch("/api/volume/master", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ volume: vol }),
      });
      loadVolume();
    } catch (e) {}
  };

  const setMute = async (muted: boolean) => {
    try {
      await fetch("/api/volume/mute", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ muted }),
      });
      loadVolume();
    } catch (e) {}
  };

  const setAppVolume = async (appName: string, vol: number) => {
    try {
      await fetch("/api/volume/app/volume", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ app_name: appName, volume: vol }),
      });
      loadVolume();
    } catch (e) {}
  };

  const setAppMute = async (appName: string, muted: boolean) => {
    try {
      await fetch("/api/volume/app/mute", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ app_name: appName, muted }),
      });
      loadVolume();
    } catch (e) {}
  };

  if (!state) return h("div", { class: "vol-loading" }, "Loading...");

  if (!state.platform_supported) {
    return h("div", { class: "vol-unsupported" }, "Volume control not supported on this platform");
  }

  const volColor = state.master_volume < 50 ? "#4caf50" : state.master_volume < 80 ? "#ff9800" : "#f44336";

  if (variant === "minimal") {
    return h("div", { class: "vol-variant minimal" },
      h("div", { class: "vol-big", style: { color: state.muted ? "#666" : volColor } },
        state.muted ? "MUTED" : `${state.master_volume.toFixed(0)}%`
      ),
      h("button", {
        class: "vol-mute-btn",
        onClick: () => setMute(!state.muted)
      }, state.muted ? "UNMUTE" : "MUTE"),
    );
  }

  if (variant === "compact") {
    return h("div", { class: "vol-variant compact" },
      h("div", { class: "vol-header" },
        h("span", { class: "vol-device" }, state.default_device_name.substring(0, 25)),
        h("button", {
          class: "vol-icon-btn",
          onClick: () => setMute(!state.muted)
        }, state.muted ? "M" : "V"),
      ),
      h("div", { class: "vol-slider-row" },
        h("input", {
          type: "range",
          min: 0,
          max: 100,
          value: state.master_volume,
          onInput: (e: Event) => setVolume(parseFloat((e.target as HTMLInputElement).value)),
          class: "vol-slider",
          style: { "--vol-pct": `${state.master_volume}%`, "--vol-color": volColor } as any,
        }),
        h("span", { class: "vol-value", style: { color: volColor } }, `${state.master_volume.toFixed(0)}%`),
      ),
      state.per_app_supported && apps.length > 0 && h("button", {
        class: "vol-apps-toggle",
        onClick: () => setShowApps(!showApps)
      }, showApps ? "Hide Apps" : `Apps (${apps.length})`),
      showApps && state.per_app_supported && h("div", { class: "vol-apps-list" },
        apps.map(app => h("div", { class: "vol-app-item", key: app.name },
          h("div", { class: "vol-app-header" },
            h("span", { class: "vol-app-name" }, app.name.substring(0, 20)),
            h("button", {
              class: "vol-app-mute",
              onClick: () => setAppMute(app.name, !app.muted)
            }, app.muted ? "M" : "V"),
          ),
          h("input", {
            type: "range",
            min: 0,
            max: 100,
            value: app.volume,
            onInput: (e: Event) => setAppVolume(app.name, parseFloat((e.target as HTMLInputElement).value)),
            class: "vol-slider vol-app-slider",
            style: { "--vol-pct": `${app.volume}%` } as any,
          }),
        ))
      ),
    );
  }

  return h("div", { class: "vol-variant detailed" },
    h("div", { class: "vol-detail-header" },
      h("div", { class: "vol-device-full" }, state.default_device_name),
      h("button", {
        class: "vol-mute-btn-large",
        onClick: () => setMute(!state.muted),
        style: { background: state.muted ? "#f44336" : "#4caf50" }
      }, state.muted ? "UNMUTE" : "MUTE"),
    ),
    h("div", { class: "vol-big-row" },
      h("div", { class: "vol-big", style: { color: state.muted ? "#666" : volColor } },
        `${state.master_volume.toFixed(1)}%`
      ),
    ),
    h("div", { class: "vol-slider-row" },
      h("input", {
        type: "range",
        min: 0,
        max: 100,
        value: state.master_volume,
        onInput: (e: Event) => setVolume(parseFloat((e.target as HTMLInputElement).value)),
        class: "vol-slider",
        style: { "--vol-pct": `${state.master_volume}%`, "--vol-color": volColor } as any,
      }),
    ),
    state.per_app_supported && h("div", { class: "vol-apps-section" },
      h("div", { class: "vol-section-title" }, `Applications (${apps.length})`),
      apps.length === 0
        ? h("div", { class: "vol-no-apps" }, "No active audio streams")
        : apps.map(app => h("div", { class: "vol-app-item-detailed", key: app.name },
            h("div", { class: "vol-app-header" },
              h("span", { class: "vol-app-name" }, app.name),
              app.pid && h("span", { class: "vol-app-pid" }, `PID: ${app.pid}`),
              h("button", {
                class: "vol-app-mute-btn",
                onClick: () => setAppMute(app.name, !app.muted),
                style: { background: app.muted ? "#f44336" : "#666" }
              }, app.muted ? "MUTED" : "MUTE"),
            ),
            h("div", { class: "vol-app-slider-row" },
              h("input", {
                type: "range",
                min: 0,
                max: 100,
                value: app.volume,
                onInput: (e: Event) => setAppVolume(app.name, parseFloat((e.target as HTMLInputElement).value)),
                class: "vol-slider vol-app-slider",
                style: { "--vol-pct": `${app.volume}%` } as any,
              }),
              h("span", { class: "vol-app-value" }, `${app.volume.toFixed(0)}%`),
            ),
          ))
    ),
  );
}
