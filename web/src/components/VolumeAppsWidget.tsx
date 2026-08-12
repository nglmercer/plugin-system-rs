import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";
import { CSSCustomProperties } from "../lib/types";
import { AppIcon } from "./AppIcon";

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
  const [platformSupported, setPlatformSupported] = useState(true);
  const variant = (settings.variant || "compact") as string;

  const loadApps = useCallback(async () => {
    try {
      const res = await fetch("/api/volume");
      const data = await res.json();
      if (data.success && data.data) {
        setPlatformSupported(data.data.state?.per_app_supported ?? false);
        setApps(data.data.apps || []);
      }
    } catch (e) {}
  }, []);

  useEffect(() => {
    loadApps();
    const interval = setInterval(loadApps, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, []);

  /**
   * Send `app_id` *and* `app_name`: the id targets one stream, the name is
   * the fallback for backends that cannot identify streams individually.
   */
  const setAppVolume = async (app: AppVolume, vol: number) => {
    // Applied locally first so the slider does not snap back to a stale value
    // before the next poll lands.
    setApps((prev) => prev.map((a) => (a.id === app.id ? { ...a, volume: vol } : a)));
    try {
      await fetch("/api/volume/app/volume", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ app_id: app.id, app_name: app.name, volume: vol }),
      });
      loadApps();
    } catch (e) {}
  };

  const setAppMute = async (app: AppVolume, muted: boolean) => {
    setApps((prev) => prev.map((a) => (a.id === app.id ? { ...a, muted } : a)));
    try {
      await fetch("/api/volume/app/mute", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ app_id: app.id, app_name: app.name, muted }),
      });
      loadApps();
    } catch (e) {}
  };

  if (!platformSupported) {
    return h(
      "div",
      { class: "vol-unsupported" },
      "Per-app volume not supported on this platform",
    );
  }

  if (apps.length === 0) {
    return h(
      "div",
      { class: "volapps-empty" },
      h("div", { class: "volapps-empty-icon" }, "A"),
      h("div", { class: "volapps-empty-text" }, "No active audio streams"),
    );
  }

  const slider = (app: AppVolume) =>
    h(
      "div",
      { class: "volapps-slider-row" },
      h("input", {
        type: "range",
        min: 0,
        max: 100,
        value: app.volume,
        onInput: (e: Event) =>
          setAppVolume(app, parseFloat((e.target as HTMLInputElement).value)),
        class: "volapps-slider",
        style: { "--vol-pct": `${app.volume}%` } as CSSCustomProperties,
      }),
      h("span", { class: "volapps-value" }, `${app.volume.toFixed(0)}%`),
    );

  if (variant === "minimal") {
    return h(
      "div",
      { class: "volapps-variant minimal" },
      h("div", { class: "volapps-count" }, `${apps.length} stream${apps.length !== 1 ? "s" : ""}`),
      apps.slice(0, 3).map((app, i) =>
        h(
          "div",
          { class: "volapps-mini-item", key: keyOf(app, i) },
          h(AppIcon, { icon: app.icon, name: app.name, size: 14 }),
          // The title identifies the stream; the app name is the same across
          // a browser's tabs and tells the user nothing here.
          h("span", { class: "volapps-mini-name", title: app.title || app.name },
            app.title || app.name),
          h(
            "span",
            {
              class: "volapps-mini-vol",
              style: { color: app.muted ? "#666" : "#4caf50" },
            },
            app.muted ? "M" : `${app.volume.toFixed(0)}%`,
          ),
        ),
      ),
      apps.length > 3 &&
        h("div", { class: "volapps-mini-more" }, `+${apps.length - 3} more`),
    );
  }

  if (variant === "compact") {
    return h(
      "div",
      { class: "volapps-variant compact" },
      h(
        "div",
        { class: "volapps-header" },
        h("span", { class: "volapps-title" }, `Audio Streams (${apps.length})`),
      ),
      h(
        "div",
        { class: "volapps-list" },
        apps.map((app, i) =>
          h(
            "div",
            { class: "volapps-item", key: keyOf(app, i) },
            h(
              "div",
              { class: "volapps-item-header" },
              h(AppIcon, { icon: app.icon, name: app.name, size: 18 }),
              h(
                "div",
                { class: "volapps-item-labels" },
                h("span", { class: "volapps-item-name", title: app.name }, app.name),
                app.title &&
                  h("span", { class: "volapps-item-title", title: app.title }, app.title),
              ),
              h(
                "button",
                {
                  class: `volapps-mute-btn ${app.muted ? "muted" : ""}`,
                  onClick: () => setAppMute(app, !app.muted),
                  title: app.muted ? "Unmute" : "Mute",
                },
                app.muted ? "M" : "V",
              ),
            ),
            slider(app),
          ),
        ),
      ),
    );
  }

  return h(
    "div",
    { class: "volapps-variant detailed" },
    h(
      "div",
      { class: "volapps-header" },
      h("span", { class: "volapps-title" }, `Active Audio Streams (${apps.length})`),
    ),
    h(
      "div",
      { class: "volapps-list detailed" },
      apps.map((app, i) =>
        h(
          "div",
          { class: "volapps-item-detailed", key: keyOf(app, i) },
          h(
            "div",
            { class: "volapps-item-header" },
            h(AppIcon, { icon: app.icon, name: app.name, size: 28 }),
            h(
              "div",
              { class: "volapps-item-info" },
              h("span", { class: "volapps-item-name", title: app.name }, app.name),
              app.title &&
                h("span", { class: "volapps-item-title", title: app.title }, app.title),
              app.pid &&
                h("span", { class: "volapps-item-pid" }, `PID: ${app.pid}`),
            ),
            h(
              "button",
              {
                class: `volapps-mute-btn-detailed ${app.muted ? "muted" : ""}`,
                onClick: () => setAppMute(app, !app.muted),
              },
              app.muted ? "UNMUTE" : "MUTE",
            ),
          ),
          slider(app),
        ),
      ),
    ),
  );
}
