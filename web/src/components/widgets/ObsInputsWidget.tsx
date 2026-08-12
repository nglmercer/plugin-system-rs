import { h } from "preact";
import { useState, useCallback } from "preact/hooks";
import { usePolling } from "../../lib/usePolling";
import { fetchObsInputs, setInputVolume, setInputMute } from "../../lib/api";
import { errorMessage } from "../../lib/format";
import { refreshMs } from "./widgetUtils";
import { MuteButton, SliderRow, WidgetEmpty, WidgetError, WidgetHead, WidgetLoading } from "./widgetParts";
import "./obsInputs.css";

interface ObsInput {
  name: string;
  kind: string;
  uuid: string;
  muted: boolean;
  volume: number;
}

export function ObsInputsWidget({ settings }: { settings: Record<string, any> }) {
  const [inputs, setInputs] = useState<ObsInput[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;

  // Rethrows so `usePolling` slows down while OBS is unreachable; the error
  // text is whatever the API reported ("not connected to OBS").
  const fetchInputs = useCallback(async () => {
    try {
      setInputs(await fetchObsInputs());
      setError(null);
    } catch (e) {
      setError(errorMessage(e, "Failed to fetch inputs"));
      throw e;
    } finally {
      setLoading(false);
    }
  }, []);

  usePolling(fetchInputs, refreshMs(settings));

  async function handleMuteToggle(inputName: string, currentlyMuted: boolean) {
    try {
      await setInputMute(inputName, !currentlyMuted);
      setInputs((prev) =>
        prev.map((inp) =>
          inp.name === inputName ? { ...inp, muted: !currentlyMuted } : inp
        )
      );
    } catch {}
  }

  async function handleVolumeChange(inputName: string, volume: number) {
    try {
      await setInputVolume(inputName, volume);
      setInputs((prev) =>
        prev.map((inp) =>
          inp.name === inputName ? { ...inp, volume } : inp
        )
      );
    } catch {}
  }

  if (loading) return h(WidgetLoading, null);
  if (error) return h(WidgetError, null, error);

  if (inputs.length === 0) {
    return h(WidgetEmpty, { icon: "I/O" }, "No inputs found");
  }

  if (variant === "minimal") {
    return h(
      "div",
      { class: "obsinput-variant minimal" },
      h(
        "div",
        { class: "widget-head" },
        h("span", { class: "widget-head-title" }, `${inputs.length} inputs`),
      ),
      inputs.slice(0, 4).map((inp) =>
        h(
          "div",
          { key: inp.name, class: "widget-mini-row" },
          h(
            "span",
            { class: "widget-mini-name" },
            inp.name.length > 12 ? inp.name.substring(0, 12) + ".." : inp.name,
          ),
          h(MuteButton, {
            muted: inp.muted,
            onToggle: () => handleMuteToggle(inp.name, inp.muted),
            labels: ["U", "M"],
          }),
        ),
      ),
      inputs.length > 4 &&
        h("div", { class: "widget-mini-more" }, `+${inputs.length - 4} more`),
    );
  }

  const detailed = variant === "detailed";

  return h(
    "div",
    { class: `obsinput-variant ${variant}` },
    h(WidgetHead, { title: "Inputs", value: String(inputs.length) }),
    h(
      "div",
      { class: "widget-list" },
      inputs.map((inp) =>
        h(
          "div",
          { key: inp.name, class: "widget-item" },
          h(
            "div",
            { class: "widget-item-head" },
            h(
              "div",
              { class: "widget-item-labels" },
              h("span", { class: "widget-item-name" }, inp.name),
              detailed ? h("span", { class: "widget-item-sub mono" }, inp.kind) : null,
            ),
            detailed
              ? h(MuteButton, {
                  kind: "pill",
                  muted: inp.muted,
                  onToggle: () => handleMuteToggle(inp.name, inp.muted),
                })
              : h(MuteButton, {
                  muted: inp.muted,
                  onToggle: () => handleMuteToggle(inp.name, inp.muted),
                  labels: ["U", "M"],
                }),
          ),
          h(SliderRow, {
            fraction: true,
            value: inp.volume,
            fill: inp.muted ? "var(--sd-text-dim)" : undefined,
            onInput: (v) => handleVolumeChange(inp.name, v),
          }),
        ),
      ),
    ),
  );
}
