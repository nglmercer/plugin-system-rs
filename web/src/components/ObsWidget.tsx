import { h } from "preact";
import { useState, useEffect, useCallback } from "preact/hooks";
import { fetchObsStatus, startStream, stopStream, startRecord, stopRecord, toggleRecordPause, toggleVirtualCam, saveReplayBuffer, connectObs } from "../lib/api";

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

  const fetchStatus = useCallback(async () => {
    try {
      const data = await fetchObsStatus();
      if (data) {
        setStatus(data);
        setError(null);
      } else {
        setError("OBS plugin not available");
      }
    } catch {
      setError("Failed to fetch OBS status");
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, settings.refreshInterval || 2000);
    return () => clearInterval(interval);
  }, [fetchStatus, settings.refreshInterval]);

  async function handleConnect() {
    try {
      await connectObs(settings.host || "127.0.0.1", settings.port || 4455, settings.password || "");
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

  if (loading) return h("div", { class: "obs-loading" }, "Loading...");
  if (error) return h("div", { class: "obs-error" }, error);
  if (!status) return h("div", { class: "obs-error" }, "No data");

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
      h("div", { class: "obs-stats-grid" },
        h("div", { class: "obs-stat" },
          h("span", { class: "obs-stat-label" }, "Scene"),
          h("span", { class: "obs-stat-value" }, status.current_scene || "-"),
        ),
        h("div", { class: "obs-stat" },
          h("span", { class: "obs-stat-label" }, "CPU"),
          h("span", { class: "obs-stat-value" }, `${status.cpu_usage.toFixed(1)}%`),
        ),
        h("div", { class: "obs-stat" },
          h("span", { class: "obs-stat-label" }, "RAM"),
          h("span", { class: "obs-stat-value" }, `${status.memory_usage.toFixed(0)} MB`),
        ),
        h("div", { class: "obs-stat" },
          h("span", { class: "obs-stat-label" }, "FPS"),
          h("span", { class: "obs-stat-value" }, status.fps.toFixed(1)),
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
