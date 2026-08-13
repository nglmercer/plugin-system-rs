import { h } from "preact";
import { useState } from "preact/hooks";
import { fetchObsMediaInputs, mediaInputAction, ObsMediaInput } from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import { errorMessage } from "../../lib/format";
import { refreshMs } from "./widgetUtils";
import { WidgetEmpty, WidgetError, WidgetHead, WidgetLoading } from "./widgetParts";
import "./obsMedia.css";

/**
 * Transport controls for media sources: play, pause, stop, restart — plus a
 * progress bar where OBS reports a position.
 */
export function ObsMediaWidget({ settings }: { settings: Record<string, any> }) {
  const [inputs, setInputs] = useState<ObsMediaInput[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;

  usePolling(async () => {
    try {
      setInputs(await fetchObsMediaInputs());
      setError(null);
    } catch (e) {
      setError(errorMessage(e, "Failed to fetch media inputs"));
      throw e;
    } finally {
      setLoading(false);
    }
  }, refreshMs(settings));

  function act(input: ObsMediaInput, action: string) {
    // The next poll reconciles; failing silently here would only flash an
    // error the user cannot act on from a tile.
    void mediaInputAction(input.name, action).catch(() => {});
  }

  if (loading) return h(WidgetLoading, null);
  if (error) return h(WidgetError, null, error);

  if (inputs.length === 0) {
    return h(WidgetEmpty, { icon: "▶" }, "No media sources");
  }

  if (variant === "minimal") {
    const first = inputs[0];
    return h(
      "div",
      { class: "media-variant minimal" },
      h("div", { class: "media-name", title: first.name }, first.name),
      h("div", { class: `media-state ${first.state}` }, first.state),
      h(
        "div",
        { class: "media-btns" },
        h(MediaButtons, { input: first, onAct: act, compact: true }),
      ),
    );
  }

  const detailed = variant === "detailed";

  return h(
    "div",
    { class: `media-variant ${variant}` },
    h(WidgetHead, { title: "Media", value: String(inputs.length) }),
    h(
      "div",
      { class: "widget-list" },
      inputs.map((input) =>
        h(
          "div",
          { class: "widget-item", key: input.name },
          h(
            "div",
            { class: "widget-item-head" },
            h(
              "div",
              { class: "widget-item-labels" },
              h("span", { class: "widget-item-name", title: input.name }, input.name),
              h("span", { class: `media-state ${input.state}` }, input.state),
            ),
            h(MediaButtons, { input, onAct: act, compact: !detailed }),
          ),
          detailed && input.duration_ms > 0
            ? h(
                "div",
                { class: "media-bar" },
                h("div", {
                  class: "media-bar-fill",
                  style: { width: `${progressPct(input)}%` },
                }),
              )
            : null,
        ),
      ),
    ),
  );
}

function progressPct(input: ObsMediaInput): number {
  if (input.duration_ms <= 0) return 0;
  return Math.min(100, Math.max(0, (input.cursor_ms / input.duration_ms) * 100));
}

function MediaButtons({
  input,
  onAct,
  compact,
}: {
  input: ObsMediaInput;
  onAct: (input: ObsMediaInput, action: string) => void;
  compact: boolean;
}) {
  const playing = input.state === "playing";
  const paused = input.state === "paused";

  return h(
    "div",
    { class: "media-controls" },
    h(
      "button",
      {
        class: `media-btn${playing ? " active" : ""}`,
        title: playing ? "Pause" : "Play",
        onClick: () => onAct(input, playing ? "pause" : "play"),
      },
      playing ? "❚❚" : "▶",
    ),
    h(
      "button",
      {
        class: "media-btn",
        title: "Stop",
        disabled: !playing && !paused,
        onClick: () => onAct(input, "stop"),
      },
      "■",
    ),
    !compact &&
      h(
        "button",
        {
          class: "media-btn",
          title: "Restart",
          onClick: () => onAct(input, "restart"),
        },
        "↺",
      ),
  );
}
