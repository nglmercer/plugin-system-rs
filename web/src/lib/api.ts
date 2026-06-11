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

export async function recordHotkey(timeoutMs: number = 5000): Promise<string> {
  return new Promise((resolve, reject) => {
    const controller = new AbortController();
    const timeout = setTimeout(() => {
      controller.abort();
      reject(new Error('Hotkey recording timed out'));
    }, timeoutMs);

    const keys = new Set<string>();
    let resolved = false;

    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      const key = e.key === 'Control' ? 'Ctrl' :
                  e.key === 'Shift' ? 'Shift' :
                  e.key === 'Alt' ? 'Alt' :
                  e.key === 'Meta' ? 'Win' :
                  e.key;
      if (key === 'Ctrl' || key === 'Shift' || key === 'Alt' || key === 'Win') {
        keys.add(key);
      } else {
        keys.add(key);
        finish();
      }
    }

    function onKeyUp(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      finish();
    }

    function finish() {
      if (resolved) return;
      resolved = true;
      clearTimeout(timeout);
      document.removeEventListener('keydown', onKeyDown);
      document.removeEventListener('keyup', onKeyUp);
      const modifiers = ['Ctrl', 'Shift', 'Alt', 'Win'];
      const mods = modifiers.filter(m => keys.has(m)).sort();
      const main = [...keys].filter(k => !modifiers.includes(k));
      resolve([...mods, ...main].join('+').toLowerCase() || 'unknown');
    }

    document.addEventListener('keydown', onKeyDown);
    document.addEventListener('keyup', onKeyUp);
  });
}
