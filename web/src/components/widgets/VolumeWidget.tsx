import { h } from "preact";
import { useState } from "preact/hooks";
import { fetchVolumeState, setMasterVolume, setMasterMute } from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import { levelColor, refreshMs } from "./widgetUtils";
import { MuteButton, SliderRow, WidgetError, WidgetLoading } from "./widgetParts";
import "./volume.css";

interface VolumeState {
  master_volume: number;
  muted: boolean;
  default_device_name: string;
  platform_supported: boolean;
}

export function VolumeWidget({ settings }: { settings: Record<string, any> }) {
  const [state, setState] = useState<VolumeState | null>(null);
  const variant = (settings.variant || "compact") as string;

  usePolling(async () => {
    const data = await fetchVolumeState();
    if (!data) return;
    setState({
      master_volume: data.state?.master_volume ?? 0,
      muted: data.state?.muted ?? false,
      default_device_name: data.state?.default_device_name ?? "",
      platform_supported: data.state?.platform_supported ?? false,
    });
  }, refreshMs(settings));

  /**
   * Applied locally first so the slider does not snap back to a stale value
   * before the next poll lands; the poll reconciles either way.
   */
  async function changeVolume(volume: number) {
    setState((prev) => (prev ? { ...prev, master_volume: volume } : prev));
    try {
      await setMasterVolume(volume);
    } catch {}
  }

  async function changeMute(muted: boolean) {
    setState((prev) => (prev ? { ...prev, muted } : prev));
    try {
      await setMasterMute(muted);
    } catch {}
  }

  if (!state) return h(WidgetLoading, null);
  if (!state.platform_supported) {
    return h(WidgetError, null, "Volume control not supported on this platform");
  }

  const fill = state.muted ? "var(--sd-text-dim)" : levelColor(state.master_volume);

  if (variant === "minimal") {
    return h(
      "div",
      { class: "vol-variant minimal" },
      h(
        "div",
        { class: "vol-big", style: { color: fill } },
        state.muted ? "MUTED" : `${state.master_volume.toFixed(0)}%`,
      ),
      h(MuteButton, { kind: "pill", muted: state.muted, onToggle: changeMute }),
    );
  }

  if (variant === "detailed") {
    return h(
      "div",
      { class: "vol-variant detailed" },
      h(
        "div",
        { class: "widget-head" },
        h("span", { class: "widget-head-title" }, state.default_device_name || "Default device"),
        h(MuteButton, { kind: "pill", muted: state.muted, onToggle: changeMute }),
      ),
      h(
        "div",
        { class: "vol-big-row" },
        h("div", { class: "vol-big", style: { color: fill } }, `${state.master_volume.toFixed(1)}%`),
      ),
      h(SliderRow, { value: state.master_volume, fill, onInput: changeVolume }),
    );
  }

  return h(
    "div",
    { class: "vol-variant compact" },
    h(
      "div",
      { class: "widget-head" },
      h(
        "span",
        { class: "widget-head-title", title: state.default_device_name },
        state.default_device_name ? state.default_device_name.substring(0, 25) : "Default device",
      ),
      h(MuteButton, { muted: state.muted, onToggle: changeMute }),
    ),
    h(SliderRow, { value: state.master_volume, fill, onInput: changeVolume }),
  );
}
