import { DashboardLayout } from './types';

const API_BASE = '/api';

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
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
  });
  const data = await res.json();
  return data.data;
}

export async function deleteProfile(id: string) {
  const res = await fetch(`${API_BASE}/profiles/${id}`, {
    method: 'DELETE',
  });
  const data = await res.json();
  return data.data;
}

export async function fetchActions() {
  const res = await fetch(`${API_BASE}/actions`);
  const data = await res.json();
  return data.data || [];
}

export async function fetchPlugins() {
  const res = await fetch(`${API_BASE}/plugins`);
  const data = await res.json();
  return data.data || [];
}

export async function reloadPlugins() {
  const res = await fetch(`${API_BASE}/plugins/reload`, {
    method: 'POST',
  });
  const data = await res.json();
  return data;
}

export async function simulateButtonPress(deviceId: string, buttonIndex: number) {
  const res = await fetch(`${API_BASE}/devices/${deviceId}/press/${buttonIndex}`, {
    method: 'POST',
  });
  const data = await res.json();
  return data;
}

export async function fetchSystemStats() {
  const res = await fetch(`${API_BASE}/system-stats`);
  const data = await res.json();
  return data.data;
}

export async function fetchPluginData(pluginName: string) {
  const res = await fetch(`${API_BASE}/plugins/${pluginName}`);
  const data = await res.json();
  return data.data;
}

export async function fetchDashboard(): Promise<DashboardLayout> {
  const res = await fetch(`${API_BASE}/dashboard`);
  const data = await res.json();
  return data.data || { widgets: [], columns: 3 };
}

export async function saveDashboard(layout: DashboardLayout): Promise<boolean> {
  const res = await fetch(`${API_BASE}/dashboard`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(layout),
  });
  const data = await res.json();
  return data.success;
}

export async function executeAction(actionName: string): Promise<string> {
  const res = await fetch(`${API_BASE}/actions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ action_name: actionName }),
  });
  const data = await res.json();
  return data.data || data.error || 'Unknown result';
}

export async function sendHotkeyCombo(combo: string): Promise<string> {
  const keys = combo.split('+').filter(Boolean);
  const res = await fetch(`${API_BASE}/hotkey/send`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ keys }),
  });
  const data = await res.json();
  return data.data?.combo || data.data || data.error || 'Sent';
}

export async function fetchInputDevices(): Promise<{ path: string; name: string }[]> {
  const res = await fetch(`${API_BASE}/hotkey/devices`);
  const data = await res.json();
  return data.data || [];
}

export async function recordHotkeyStart(timeoutMs: number = 20000): Promise<{ session_id: string; current_combo: string }> {
  const res = await fetch(`${API_BASE}/hotkey/record`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ timeout_ms: timeoutMs }),
  });
  const data = await res.json();
  if (!data.success) throw new Error(data.error || 'Failed to start recording');
  return data.data;
}

export async function recordHotkeyStatus(sessionId: string): Promise<{ status: string; current_combo: string; result: string | null }> {
  const res = await fetch(`${API_BASE}/hotkey/record/${sessionId}`);
  const data = await res.json();
  if (!data.success) throw new Error(data.error || 'Session not found');
  return data.data;
}

export async function recordHotkeyCancel(sessionId: string): Promise<void> {
  await fetch(`${API_BASE}/hotkey/record/${sessionId}/cancel`, { method: 'POST' });
}
