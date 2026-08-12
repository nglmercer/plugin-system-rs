import { h } from "preact";
import { useState } from "preact/hooks";
import { fetchVolumeState, setAppVolume, setAppMute } from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import { AppIcon } from "./AppIcon";
import { refreshMs } from "./widgetUtils";
import { MuteButton, SliderRow, WidgetEmpty, WidgetError, WidgetHead, WidgetLoading } from "./widgetParts";
import "./volumeApps.css";

/**
 * Per-application audio streams.
 *
 * A stream is not an application: a browser plays one per tab, and they are
 * controlled independently. Entries are therefore keyed and addressed by
 * `id` — the backend's stream handle — with the application name and the
 * stream title shown as two separate lines, which is what distinguishes three
 * simultaneous Firefox streams from one another.
 */

interface AppVolume {
  /** Backend handle for this exact stream. */
  id: string;
  name: string;
  /** What the stream is playing, e.g. a tab title. May be empty. */
  title?: string;
  /** Freedesktop icon name; resolved by the host. May be empty. */
  icon?: string;
  volume: number;
  muted: boolean;
  pid: number | null;
}

/** Address a stream, tolerating a backend that reports no id. */
function keyOf(app: AppVolume, index: number): string {
  return app.id || `${app.name}:${index}`;
}

export function VolumeAppsWidget({ settings }: { settings: Record<string, any> }) {
  const [apps, setApps] = useState<AppVolume[]>([]);
  const [supported, setSupported] = useState(true);
  const [loading, setLoading] = useState(true);
  const variant = (settings.variant || "compact") as string;

  usePolling(async () => {
    const data = await fetchVolumeState();
    if (data) {
      setSupported(data.state?.per_app_supported ?? false);
      setApps(data.apps || []);
    }
    setLoading(false);
  }, refreshMs(settings));

  function patch(id: string, changes: Partial<AppVolume>) {
    setApps((prev) => prev.map((a) => (a.id === id ? { ...a, ...changes } : a)));
  }

  // Applied locally first so the slider does not snap back to a stale value
  // before the next poll lands.
  async function changeVolume(app: AppVolume, volume: number) {
    patch(app.id, { volume });
    try {
      await setAppVolume({ id: app.id, name: app.name }, volume);
    } catch {}
  }

  async function changeMute(app: AppVolume, muted: boolean) {
    patch(app.id, { muted });
    try {
      await setAppMute({ id: app.id, name: app.name }, muted);
    } catch {}
  }

  if (!supported) {
    return h(WidgetError, null, "Per-app volume not supported on this platform");
  }
  if (loading && apps.length === 0) return h(WidgetLoading, null);

  if (apps.length === 0) {
    return h(WidgetEmpty, { icon: "♪" }, "No active audio streams");
  }

  const streamCount = `${apps.length} stream${apps.length !== 1 ? "s" : ""}`;

  if (variant === "minimal") {
    return h(
      "div",
      { class: "volapps-variant minimal" },
      h("div", { class: "widget-head" }, h("span", { class: "widget-head-title" }, streamCount)),
      apps.slice(0, 3).map((app, i) =>
        h(
          "div",
          { class: "widget-mini-row", key: keyOf(app, i) },
          h(AppIcon, { icon: app.icon, name: app.name, size: 14 }),
          // The title identifies the stream; the app name is the same across
          // a browser's tabs and tells the user nothing here.
          h(
            "span",
            { class: "widget-mini-name", title: app.title || app.name },
            app.title || app.name,
          ),
          h(
            "span",
            { class: `widget-mini-value${app.muted ? " muted" : ""}` },
            app.muted ? "M" : `${app.volume.toFixed(0)}%`,
          ),
        ),
      ),
      apps.length > 3 && h("div", { class: "widget-mini-more" }, `+${apps.length - 3} more`),
    );
  }

  const detailed = variant === "detailed";

  return h(
    "div",
    { class: `volapps-variant ${variant}` },
    h(WidgetHead, {
      title: detailed ? "Active Audio Streams" : "Audio Streams",
      value: String(apps.length),
    }),
    h(
      "div",
      { class: "widget-list" },
      apps.map((app, i) =>
        h(
          "div",
          { class: "widget-item", key: keyOf(app, i) },
          h(
            "div",
            { class: "widget-item-head" },
            h(AppIcon, { icon: app.icon, name: app.name, size: detailed ? 28 : 18 }),
            h(
              "div",
              { class: "widget-item-labels" },
              h("span", { class: "widget-item-name", title: app.name }, app.name),
              app.title && h("span", { class: "widget-item-sub", title: app.title }, app.title),
              detailed && app.pid
                ? h("span", { class: "widget-item-sub mono" }, `PID: ${app.pid}`)
                : null,
            ),
            detailed
              ? h(MuteButton, { kind: "pill", muted: app.muted, onToggle: (m) => changeMute(app, m) })
              : h(MuteButton, { muted: app.muted, onToggle: (m) => changeMute(app, m) }),
          ),
          h(SliderRow, {
            value: app.volume,
            fill: app.muted ? "var(--sd-text-dim)" : undefined,
            onInput: (v) => changeVolume(app, v),
          }),
        ),
      ),
    ),
  );
}
