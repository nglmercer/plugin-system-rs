import { h } from "preact";
import { useState, useCallback, useRef } from "preact/hooks";
import { fetchObsStatus, startStream, stopStream, startRecord, stopRecord, toggleRecordPause, toggleVirtualCam, saveReplayBuffer, connectObs } from "../../lib/api";
import { usePolling } from "../../lib/usePolling";
import { errorMessage } from "../../lib/format";
import { refreshMs } from "./widgetUtils";
import { WidgetError, WidgetLoading } from "./widgetParts";
import "./obs.css";

interface ObsStatus {
  connected: boolean;
  host: string;
  port: number;
  stream_active: boolean;
  record_active: boolean;
  record_paused: boolean;
  virtual_cam_active: boolean;
  replay_buffer_active: boolean;
  current_scene: string;
  studio_mode: boolean;
  cpu_usage: number;
  memory_usage: number;
  fps: number;
}

export function ObsWidget({ settings }: { settings: Record<string, any> }) {
  const [status, setStatus] = useState<ObsStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const variant = (settings.variant || "compact") as string;
  /**
   * Auto-connect is attempted once per mount, never per poll: a dashboard
   * widget should come up live without a click, but retrying every two
   * seconds would hammer a deliberately-closed OBS forever.
   */
  const autoConnectTried = useRef(false);

  const fetchStatus = useCallback(async () => {
    try {
      const data = await fetchObsStatus();
      if (data) {
        setStatus(data);
        setError(null);
        if (!data.connected && !autoConnectTried.current && settings.autoConnect !== false) {
          autoConnectTried.current = true;
          void handleConnect();
        }
      } else {
        // The plugin answers even while disconnected, so a null here means
        // the plugin is genuinely missing — not merely not connected to OBS.
        setError("OBS plugin not loaded");
      }
    } catch (e) {
      // The server's own message says what actually failed ("not connected to
      // OBS", "obs plugin unavailable"); a fixed string here hid all of it
      // behind one wrong-looking diagnosis.
      setError(errorMessage(e, "Failed to fetch OBS status"));
      // Rethrown so `usePolling` backs off while OBS is down instead of
      // knocking every two seconds forever.
      throw e;
    }
    setLoading(false);
  }, []);

  usePolling(fetchStatus, refreshMs(settings));

  async function handleConnect() {
    // A manual click should retry even after auto-connect gave up.
    autoConnectTried.current = true;
    try {
      const ok = await connectObs(
        settings.host || "127.0.0.1",
        settings.port || 4455,
        settings.password || "",
      );
      if (!ok) setError("Could not connect to OBS");
      fetchStatus();
    } catch {
      setError("Connection failed");
    }
  }

  async function handleStreamToggle() {
    if (!status) return;
    try {
      if (status.stream_active) {
        await stopStream();
      } else {
        await startStream();
      }
      fetchStatus();
    } catch {}
  }

  async function handleRecordToggle() {
    if (!status) return;
    try {
      if (status.record_active) {
        await stopRecord();
      } else {
        await startRecord();
      }
      fetchStatus();
    } catch {}
  }

  async function handleRecordPause() {
    try {
      await toggleRecordPause();
      fetchStatus();
    } catch {}
  }

  async function handleVirtualCam() {
    try {
      await toggleVirtualCam();
      fetchStatus();
    } catch {}
  }

  async function handleSaveReplay() {
    try {
      await saveReplayBuffer();
    } catch {}
  }

  if (loading) return h(WidgetLoading, null);
  if (error) return h(WidgetError, null, error);
  if (!status) return h(WidgetError, null, "No data");

  if (variant === "minimal") {
    return h("div", { class: "obs-variant minimal" },
      h("div", { class: "obs-status-row" },
        h("span", { class: `obs-dot ${status.connected ? "green" : "red"}` }),
        h("span", { class: "obs-conn-label" }, status.connected ? "Connected" : "Disconnected"),
      ),
      h("div", { class: "obs-status-row" },
        h("span", { class: `obs-dot ${status.stream_active ? "red" : ""}` }),
        h("span", null, "Stream"),
      ),
      h("div", { class: "obs-status-row" },
        h("span", { class: `obs-dot ${status.record_active ? "red" : ""}` }),
        h("span", null, "Record"),
      ),
      !status.connected && h("button", { class: "obs-btn", onClick: handleConnect }, "Connect"),
    );
  }

  if (variant === "detailed") {
    return h("div", { class: "obs-variant detailed" },
      h("div", { class: "obs-detail-header" },
        h("span", { class: `obs-dot ${status.connected ? "green" : "red"}` }),
        h("span", { class: "obs-conn-label" }, status.connected ? `${status.host}:${status.port}` : "Disconnected"),
        !status.connected && h("button", { class: "obs-btn-sm", onClick: handleConnect }, "Connect"),
      ),
      h("div", { class: "obs-controls-grid" },
        h("button", {
          class: `obs-ctrl-btn ${status.stream_active ? "active" : ""}`,
          onClick: handleStreamToggle,
          disabled: !status.connected,
        }, status.stream_active ? "Stop Stream" : "Start Stream"),
        h("button", {
          class: `obs-ctrl-btn ${status.record_active ? "active" : ""}`,
          onClick: handleRecordToggle,
          disabled: !status.connected,
        }, status.record_active ? "Stop Record" : "Start Record"),
        h("button", {
          class: `obs-ctrl-btn ${status.virtual_cam_active ? "active" : ""}`,
          onClick: handleVirtualCam,
          disabled: !status.connected,
        }, status.virtual_cam_active ? "Stop VCam" : "Start VCam"),
        h("button", {
          class: "obs-ctrl-btn",
          onClick: handleSaveReplay,
          disabled: !status.connected,
        }, "Save Replay"),
      ),
      status.record_active && h("div", { class: "obs-pause-row" },
        h("button", {
          class: `obs-ctrl-btn small ${status.record_paused ? "paused" : ""}`,
          onClick: handleRecordPause,
        }, status.record_paused ? "Resume" : "Pause"),
      ),
      h("div", { class: "stat-grid" },
        h("div", { class: "stat-cell" },
          h("span", { class: "stat-label" }, "Scene"),
          h("span", { class: "stat-value" }, status.current_scene || "-"),
        ),
        h("div", { class: "stat-cell" },
          h("span", { class: "stat-label" }, "CPU"),
          h("span", { class: "stat-value" }, `${status.cpu_usage.toFixed(1)}%`),
        ),
        h("div", { class: "stat-cell" },
          h("span", { class: "stat-label" }, "RAM"),
          h("span", { class: "stat-value" }, `${status.memory_usage.toFixed(0)} MB`),
        ),
        h("div", { class: "stat-cell" },
          h("span", { class: "stat-label" }, "FPS"),
          h("span", { class: "stat-value" }, status.fps.toFixed(1)),
        ),
      ),
    );
  }

  return h("div", { class: "obs-variant compact" },
    h("div", { class: "obs-compact-header" },
      h("span", { class: `obs-dot ${status.connected ? "green" : "red"}` }),
      h("span", { class: "obs-scene-name" }, status.current_scene || (status.connected ? "No scene" : "Disconnected")),
    ),
    h("div", { class: "obs-compact-controls" },
      h("button", {
        class: `obs-toggle-btn ${status.stream_active ? "active" : ""}`,
        onClick: handleStreamToggle,
        disabled: !status.connected,
        title: "Stream",
      }, "STR"),
      h("button", {
        class: `obs-toggle-btn ${status.record_active ? "active" : ""}`,
        onClick: handleRecordToggle,
        disabled: !status.connected,
        title: "Record",
      }, "REC"),
      h("button", {
        class: `obs-toggle-btn ${status.virtual_cam_active ? "active" : ""}`,
        onClick: handleVirtualCam,
        disabled: !status.connected,
        title: "Virtual Camera",
      }, "VC"),
    ),
    !status.connected && h("button", { class: "obs-btn", onClick: handleConnect }, "Connect"),
  );
}
