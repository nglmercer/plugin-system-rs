import { h } from "preact";
import { useState } from "preact/hooks";
import {
  fetchObsScenes,
  fetchObsStatus,
  saveObsScreenshot,
  setCurrentScene,
  setPreviewScene,
  setStudioMode,
  triggerStudioTransition,
  ObsStatus,
} from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import { errorMessage } from "../../lib/format";
import { refreshMs } from "./widgetUtils";
import { WidgetError, WidgetLoading } from "./widgetParts";
import "./obsStudio.css";

/**
 * Studio mode: preview beside program, with the transition that promotes one
 * to the other, and a snapshot of the program scene.
 */
export function ObsStudioWidget({ settings }: { settings: Record<string, any> }) {
  const [status, setStatus] = useState<ObsStatus | null>(null);
  const [scenes, setScenes] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const variant = (settings.variant || "compact") as string;

  usePolling(async () => {
    try {
      const data = await fetchObsStatus();
      if (!data) {
        setError("OBS plugin not loaded");
        return;
      }
      setStatus(data);
      setError(null);
      if (data.connected && variant !== "minimal") {
        const list = await fetchObsScenes();
        setScenes((list?.scenes ?? []).map((s) => s.name));
      }
    } catch (e) {
      setError(errorMessage(e, "Failed to fetch OBS status"));
      throw e;
    } finally {
      setLoading(false);
    }
  }, refreshMs(settings));

  async function run(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
      const data = await fetchObsStatus();
      if (data) setStatus(data);
    } catch {}
    setBusy(false);
  }

  if (loading) return h(WidgetLoading, null);
  if (error) return h(WidgetError, null, error);
  if (!status) return h(WidgetError, null, "No data");

  const studio = status.studio_mode;

  if (variant === "minimal") {
    return h(
      "div",
      { class: "studio-variant minimal" },
      h(
        "button",
        {
          class: `studio-toggle${studio ? " active" : ""}`,
          disabled: !status.connected || busy,
          onClick: () => run(() => setStudioMode(!studio)),
        },
        studio ? "Studio ON" : "Studio OFF",
      ),
      h(
        "div",
        { class: "studio-scene-line" },
        studio ? `${status.preview_scene || "?"} → ${status.current_scene || "?"}` : status.current_scene || "No scene",
      ),
    );
  }

  const transition = h(
    "button",
    {
      class: "studio-transition-btn",
      disabled: !status.connected || !studio || busy,
      title: "Promote preview to program",
      onClick: () => run(() => triggerStudioTransition()),
    },
    "Transition",
  );

  if (variant === "detailed") {
    return h(
      "div",
      { class: "studio-variant detailed" },
      h(
        "div",
        { class: "widget-head" },
        h("span", { class: "widget-head-title" }, "Studio Mode"),
        h(
          "button",
          {
            class: `studio-toggle${studio ? " active" : ""}`,
            disabled: !status.connected || busy,
            onClick: () => run(() => setStudioMode(!studio)),
          },
          studio ? "ON" : "OFF",
        ),
      ),
      h(
        "div",
        { class: "studio-pair" },
        h("div", { class: "studio-slot" },
          h("span", { class: "studio-slot-label" }, "Preview"),
          h("span", { class: "studio-slot-name" }, studio ? status.preview_scene || "—" : "—"),
        ),
        h("div", { class: "studio-slot" },
          h("span", { class: "studio-slot-label" }, "Program"),
          h("span", { class: "studio-slot-name" }, status.current_scene || "—"),
        ),
      ),
      h("div", { class: "studio-actions" },
        transition,
        h(
          "button",
          {
            class: "studio-transition-btn",
            disabled: !status.connected || busy,
            title: "Save a screenshot of the program scene",
            onClick: () => run(() => saveObsScreenshot()),
          },
          "Snapshot",
        ),
      ),
      studio &&
        scenes.length > 0 &&
        h(
          "div",
          { class: "widget-list" },
          scenes.map((name) =>
            h(
              "button",
              {
                class: `studio-scene-btn${name === status.preview_scene ? " active" : ""}`,
                title: "Set as preview",
                disabled: busy,
                onClick: () => run(() => setPreviewScene(name)),
              },
              name,
            ),
          ),
        ),
      !studio &&
        h(
          "p",
          { class: "studio-hint" },
          "Enable studio mode to edit the preview before taking it live.",
        ),
    );
  }

  return h(
    "div",
    { class: "studio-variant compact" },
    h(
      "div",
      { class: "widget-head" },
      h("span", { class: "widget-head-title" }, "Studio"),
      h(
        "button",
        {
          class: `studio-toggle${studio ? " active" : ""}`,
          disabled: !status.connected || busy,
          onClick: () => run(() => setStudioMode(!studio)),
        },
        studio ? "ON" : "OFF",
      ),
    ),
    h(
      "div",
      { class: "studio-scene-line" },
      studio
        ? `${status.preview_scene || "?"} → ${status.current_scene || "?"}`
        : status.current_scene || "No scene",
    ),
    transition,
    // A program change is also available without studio mode, so the widget
    // stays useful when it is off: click to send a scene straight to program.
    !studio &&
      h(ScenePicker, {
        scenes,
        current: status.current_scene,
        disabled: busy || !status.connected,
        onPick: (name) => run(() => setCurrentScene(name)),
      }),
  );
}

function ScenePicker({
  scenes,
  current,
  disabled,
  onPick,
}: {
  scenes: string[];
  current: string;
  disabled: boolean;
  onPick: (name: string) => void;
}) {
  if (scenes.length === 0) return null;
  return h(
    "select",
    {
      class: "studio-select",
      value: current,
      disabled,
      onChange: (e: Event) => onPick((e.target as HTMLSelectElement).value),
    },
    scenes.map((name) => h("option", { key: name, value: name }, name)),
  );
}
