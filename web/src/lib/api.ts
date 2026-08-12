import { DashboardLayout, PluginStatus } from "./types";

const API_BASE = "/api";

export async function fetchDevices() {
  const res = await fetch(`${API_BASE}/devices`);
  const data = await res.json();
  return data.data || [];
}

export async function fetchProfiles() {
  const res = await fetch(`${API_BASE}/profiles`);
  const data = await res.json();
  return data.data || [];
}

export async function createProfile(name: string) {
  const res = await fetch(`${API_BASE}/profiles`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name }),
  });
  const data = await res.json();
  return data.data;
}

export async function deleteProfile(id: string) {
  const res = await fetch(`${API_BASE}/profiles/${id}`, {
    method: "DELETE",
  });
  const data = await res.json();
  return data.data;
}

export async function fetchActions() {
  const res = await fetch(`${API_BASE}/actions`);
  const data = await res.json();
  return data.data || [];
}

export async function fetchPlugins(): Promise<PluginStatus[]> {
  const res = await fetch(`${API_BASE}/plugins`);
  const data = await res.json();
  return (data.data || []) as PluginStatus[];
}

async function readPluginResponse<T>(res: Response): Promise<T> {
  const data = await res.json();
  if (!res.ok || !data.success) {
    throw new Error(data.error || "Plugin request failed");
  }
  return data.data as T;
}

export async function refreshPlugins(): Promise<PluginStatus[]> {
  const res = await fetch(`${API_BASE}/plugins/refresh`, {
    method: "POST",
  });
  await readPluginResponse<unknown>(res);
  return fetchPlugins();
}

/**
 * A plugin's sidecar manifest, sent alongside the binary on install.
 *
 * The server grants exactly the capabilities this manifest lists, and only
 * after the caller has acknowledged each one by name — so the manifest has to
 * travel with the upload rather than being discovered afterwards.
 */
export interface PluginManifestUpload {
  /** Raw JSON text of the manifest, exactly as it will be stored. */
  json: string;
  /** Capability names read out of that JSON, for the user to confirm. */
  capabilities: string[];
}

/** Read a `.manifest.json` file and pull out its capability list. */
export async function readPluginManifest(file: File): Promise<PluginManifestUpload> {
  const json = await file.text();
  let parsed: { capabilities?: unknown };
  try {
    parsed = JSON.parse(json);
  } catch (e) {
    throw new Error(`Manifest is not valid JSON: ${(e as Error).message}`);
  }
  const capabilities = Array.isArray(parsed.capabilities)
    ? parsed.capabilities.filter((c): c is string => typeof c === "string")
    : [];
  return { json, capabilities };
}

function appendManifest(formData: FormData, manifest?: PluginManifestUpload) {
  if (!manifest) return;
  formData.append("manifest", manifest.json);
  // Sent as one field per capability so a capability name containing a comma
  // could never smuggle in a second acknowledgement.
  for (const capability of manifest.capabilities) {
    formData.append("acknowledge_capability", capability);
  }
}

export async function uploadPluginFile(
  file: File,
  enabled: boolean,
  manifest?: PluginManifestUpload,
): Promise<PluginStatus> {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("enabled", String(enabled));
  appendManifest(formData, manifest);

  const res = await fetch(`${API_BASE}/plugins/upload`, {
    method: "POST",
    body: formData,
  });
  return readPluginResponse<PluginStatus>(res);
}

export async function updatePluginFile(
  pluginName: string,
  file: File,
  enabled?: boolean,
  manifest?: PluginManifestUpload,
): Promise<PluginStatus> {
  const formData = new FormData();
  formData.append("file", file);
  if (enabled !== undefined) {
    formData.append("enabled", String(enabled));
  }
  appendManifest(formData, manifest);

  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(pluginName)}/update`, {
    method: "POST",
    body: formData,
  });
  return readPluginResponse<PluginStatus>(res);
}

export async function uninstallPlugin(pluginName: string): Promise<void> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(pluginName)}`, {
    method: "DELETE",
  });
  await readPluginResponse<string>(res);
}

export async function reloadPlugins() {
  const res = await fetch(`${API_BASE}/plugins/reload`, {
    method: "POST",
  });
  const data = await res.json();
  return data;
}

export async function setPluginEnabled(pluginName: string, enabled: boolean): Promise<PluginStatus> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(pluginName)}/enabled`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ enabled }),
  });
  const data = await res.json();
  if (!res.ok || !data.success) {
    throw new Error(data.error || "Failed to update plugin");
  }
  return data.data as PluginStatus;
}

export async function simulateButtonPress(
  deviceId: string,
  buttonIndex: number,
) {
  const res = await fetch(
    `${API_BASE}/devices/${deviceId}/press/${buttonIndex}`,
    {
      method: "POST",
    },
  );
  const data = await res.json();
  return data;
}

export async function fetchSystemStats() {
  const res = await fetch(`${API_BASE}/system-stats`);
  const data = await res.json();
  return data.data;
}

export async function fetchLocalIp() {
  const res = await fetch(`${API_BASE}/local-ip`);
  const data = await res.json();
  return data.data;
}

export async function fetchPluginData(pluginName: string) {
  const res = await fetch(`${API_BASE}/plugins/${pluginName}`);
  const data = await res.json();
  return data.data;
}

/**
 * Invoke a command on a plugin that has no typed endpoint of its own.
 *
 * Throws on a plugin-reported failure so callers can surface the plugin's own
 * message rather than rendering an empty success.
 */
export async function callPluginCommand<T = any>(
  pluginName: string,
  method: string,
  args: Record<string, any> = {},
): Promise<T> {
  const res = await fetch(`${API_BASE}/plugins/${encodeURIComponent(pluginName)}/command`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ method, args }),
  });
  const data = await res.json();
  if (!res.ok || !data.success) {
    throw new Error(data.error || `Plugin command '${method}' failed`);
  }
  return data.data as T;
}

export interface TimerSnapshot {
  name: string;
  seconds: number;
  remaining: number;
  remaining_ms: number;
  expired: boolean;
}

export async function fetchTimers(): Promise<TimerSnapshot[]> {
  const data = await fetchPluginData("timer");
  return (data?.data?.timers ?? []) as TimerSnapshot[];
}

export async function startTimer(name: string, seconds: number) {
  return callPluginCommand("timer", "start", { name, seconds });
}

export async function stopTimer(name: string) {
  return callPluginCommand("timer", "stop", { name });
}

export async function fetchDashboard(): Promise<DashboardLayout> {
  const res = await fetch(`${API_BASE}/dashboard`);
  const data = await res.json();
  return data.data || { widgets: [], columns: 3 };
}

export async function saveDashboard(layout: DashboardLayout): Promise<boolean> {
  const res = await fetch(`${API_BASE}/dashboard`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(layout),
  });
  const data = await res.json();
  return data.success;
}

export async function executeAction(actionName: string): Promise<string> {
  const res = await fetch(`${API_BASE}/actions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action_name: actionName }),
  });
  const data = await res.json();
  return data.data || data.error || "Unknown result";
}

export async function openUrl(url: string): Promise<string> {
  const res = await fetch(`${API_BASE}/actions/open-url`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url }),
  });
  const data = await res.json();
  return data.data || data.error || "Unknown result";
}

export async function sendHotkeyCombo(combo: string): Promise<string> {
  const keys = combo.split("+").filter(Boolean);
  const res = await fetch(`${API_BASE}/hotkey/send`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ keys }),
  });
  const data = await res.json();
  return data.data?.combo || data.data || data.error || "Sent";
}

export async function recordHotkey(timeoutMs: number = 3000): Promise<string> {
  const res = await fetch(`${API_BASE}/hotkey/record`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ timeout_ms: timeoutMs }),
  });
  const data = await res.json();
  if (!data.success) throw new Error(data.error || "Recording failed");
  return data.data.combo;
}

export async function resetHotkeyRecording(): Promise<void> {
  await fetch(`${API_BASE}/hotkey/record/reset`, { method: "POST" });
}

export async function fetchVolumeState() {
  const res = await fetch(`${API_BASE}/volume`);
  const data = await res.json();
  return data.data;
}

export async function setMasterVolume(volume: number) {
  const res = await fetch(`${API_BASE}/volume/master`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ volume }),
  });
  const data = await res.json();
  return data.success;
}

export async function setMasterMute(muted: boolean) {
  const res = await fetch(`${API_BASE}/volume/mute`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ muted }),
  });
  const data = await res.json();
  return data.success;
}

export async function fetchAppVolumes() {
  const res = await fetch(`${API_BASE}/volume/apps`);
  const data = await res.json();
  return data.data || [];
}

export async function setAppVolume(appName: string, volume: number) {
  const res = await fetch(`${API_BASE}/volume/app/volume`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ app_name: appName, volume }),
  });
  const data = await res.json();
  return data.success;
}

export async function setAppMute(appName: string, muted: boolean) {
  const res = await fetch(`${API_BASE}/volume/app/mute`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ app_name: appName, muted }),
  });
  const data = await res.json();
  return data.success;
}

export async function fetchObsStatus() {
  // Goes through `obsGet` like every other OBS read. Reading `data.data`
  // without checking `success` turned every real failure — OBS closed, wrong
  // password, plugin disabled — into `undefined`, which the widget rendered as
  // "OBS plugin not loaded" no matter what had actually gone wrong.
  return obsGet<any>("/obs/status", null);
}

export async function connectObs(host: string, port: number, password: string) {
  const res = await fetch(`${API_BASE}/obs/connect`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ host, port, password: password || undefined }),
  });
  const data = await res.json();
  return data.success;
}

export async function disconnectObs() {
  const res = await fetch(`${API_BASE}/obs/disconnect`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function startStream() {
  const res = await fetch(`${API_BASE}/obs/stream/start`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function stopStream() {
  const res = await fetch(`${API_BASE}/obs/stream/stop`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function startRecord() {
  const res = await fetch(`${API_BASE}/obs/record/start`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function stopRecord() {
  const res = await fetch(`${API_BASE}/obs/record/stop`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function toggleRecordPause() {
  const res = await fetch(`${API_BASE}/obs/record/pause`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

// OBS endpoints answer `{success: false, error}` whenever OBS is closed or
// not connected yet. Returning `undefined` there hid the reason from the
// widgets, so surface the server's message as a thrown Error instead.
async function obsGet<T>(path: string, fallback: T): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  const data = await res.json();
  if (!data.success) throw new Error(data.error || "OBS request failed");
  return (data.data ?? fallback) as T;
}

export async function fetchObsScenes() {
  return obsGet<{ current_scene: string; scenes: { name: string; index: number }[] }>(
    "/obs/scenes",
    { current_scene: "", scenes: [] },
  );
}

export async function setCurrentScene(sceneName: string) {
  const res = await fetch(`${API_BASE}/obs/scenes/current`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ scene_name: sceneName }),
  });
  const data = await res.json();
  return data.success;
}

export async function fetchObsInputs() {
  return obsGet<any[]>("/obs/inputs", []);
}

export async function setInputVolume(inputName: string, volume: number) {
  const res = await fetch(`${API_BASE}/obs/inputs/volume`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ input_name: inputName, volume }),
  });
  const data = await res.json();
  return data.success;
}

export async function setInputMute(inputName: string, muted: boolean) {
  const res = await fetch(`${API_BASE}/obs/inputs/mute`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ input_name: inputName, muted }),
  });
  const data = await res.json();
  return data.success;
}

export async function toggleVirtualCam() {
  const res = await fetch(`${API_BASE}/obs/virtualcam/toggle`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function saveReplayBuffer() {
  const res = await fetch(`${API_BASE}/obs/replay/save`, { method: "POST" });
  const data = await res.json();
  return data.success;
}

export async function fetchObsTransitions() {
  return obsGet<any[]>("/obs/transitions", []);
}

export async function setTransition(name: string) {
  const res = await fetch(`${API_BASE}/obs/transitions/current`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name }),
  });
  const data = await res.json();
  return data.success;
}

export async function fetchObsSceneItems(sceneName: string) {
  return obsGet<any[]>(`/obs/scene-items?scene_name=${encodeURIComponent(sceneName)}`, []);
}

export async function setSceneItemEnabled(sceneName: string, itemId: number, enabled: boolean) {
  const res = await fetch(`${API_BASE}/obs/scene-item/enabled`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ scene_name: sceneName, item_id: itemId, enabled }),
  });
  const data = await res.json();
  return data.success;
}
