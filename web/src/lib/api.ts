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
    const timeout = setTimeout(() => {
      cleanup();
      reject(new Error('Hotkey recording timed out'));
    }, timeoutMs);

    const pressed = new Set<string>();
    let hasMainKey = false;
    let cleaned = false;

    const MODIFIER_MAP: Record<string, string> = {
      ControlLeft: 'Ctrl', ControlRight: 'Ctrl',
      ShiftLeft: 'Shift', ShiftRight: 'Shift',
      AltLeft: 'Alt', AltRight: 'Alt',
      MetaLeft: 'Win', MetaRight: 'Win',
    };

    const CODE_MAP: Record<string, string> = {
      KeyA: 'A', KeyB: 'B', KeyC: 'C', KeyD: 'D', KeyE: 'E',
      KeyF: 'F', KeyG: 'G', KeyH: 'H', KeyI: 'I', KeyJ: 'J',
      KeyK: 'K', KeyL: 'L', KeyM: 'M', KeyN: 'N', KeyO: 'O',
      KeyP: 'P', KeyQ: 'Q', KeyR: 'R', KeyS: 'S', KeyT: 'T',
      KeyU: 'U', KeyV: 'V', KeyW: 'W', KeyX: 'X', KeyY: 'Y', KeyZ: 'Z',
    };

    for (let i = 0; i <= 9; i++) CODE_MAP[`Digit${i}`] = String(i);

    function keyName(e: KeyboardEvent): string | null {
      if (MODIFIER_MAP[e.code]) return MODIFIER_MAP[e.code];
      if (CODE_MAP[e.code]) return CODE_MAP[e.code];
      const c = e.key;
      if (c.length === 1) return c.toUpperCase();
      return null;
    }

    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      const name = keyName(e);
      if (!name) return;
      pressed.add(name);
      if (!MODIFIER_MAP[e.code]) hasMainKey = true;
    }

    function onKeyUp(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      if (!hasMainKey) return;
      const mods: string[] = [];
      if (e.ctrlKey) mods.push('Ctrl');
      if (e.shiftKey) mods.push('Shift');
      if (e.altKey) mods.push('Alt');
      if (e.metaKey) mods.push('Win');
      const mains = [...pressed].filter(k => !['Ctrl', 'Shift', 'Alt', 'Win'].includes(k));
      if (mains.length > 0) {
        finish([...new Set([...mods, ...mains])].join('+').toLowerCase());
      }
    }

    function finish(combo: string) {
      if (cleaned) return;
      cleaned = true;
      clearTimeout(timeout);
      document.removeEventListener('keydown', onKeyDown);
      document.removeEventListener('keyup', onKeyUp);
      resolve(combo);
    }

    function cleanup() {
      cleaned = true;
      document.removeEventListener('keydown', onKeyDown);
      document.removeEventListener('keyup', onKeyUp);
    }

    document.addEventListener('keydown', onKeyDown);
    document.addEventListener('keyup', onKeyUp);
  });
}

export async function sendHotkeyCombo(combo: string): Promise<string> {
  const keys = combo.split('+').filter(Boolean);
  const res = await fetch(`${API_BASE}/hotkey/send`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ keys }),
  });
  const data = await res.json();
  if (data.success && data.data) {
    simulateBrowserKeys(data.data.keys || keys);
    return data.data.combo || 'Sent';
  }
  return data.data || data.error || 'Sent';
}

const MODIFIER_CODE: Record<string, string> = {
  ctrl: 'ControlLeft', shift: 'ShiftLeft', alt: 'AltLeft', win: 'MetaLeft',
};

const KEY_CODE: Record<string, string> = {
  a: 'KeyA', b: 'KeyB', c: 'KeyC', d: 'KeyD', e: 'KeyE',
  f: 'KeyF', g: 'KeyG', h: 'KeyH', i: 'KeyI', j: 'KeyJ',
  k: 'KeyK', l: 'KeyL', m: 'KeyM', n: 'KeyN', o: 'KeyO',
  p: 'KeyP', q: 'KeyQ', r: 'KeyR', s: 'KeyS', t: 'KeyT',
  u: 'KeyU', v: 'KeyV', w: 'KeyW', x: 'KeyX', y: 'KeyY', z: 'KeyZ',
};

for (let i = 0; i <= 9; i++) KEY_CODE[String(i)] = `Digit${i}`;

function simulateBrowserKeys(keys: string[]) {
  const mods = keys.filter(k => MODIFIER_CODE[k.toLowerCase()]);
  const mains = keys.filter(k => !MODIFIER_CODE[k.toLowerCase()]);

  function dispatch(type: 'keydown' | 'keyup', key: string, code: string, modifiers: Record<string, boolean> = {}) {
    document.dispatchEvent(new KeyboardEvent(type, {
      key,
      code,
      ctrlKey: modifiers.ctrl || false,
      shiftKey: modifiers.shift || false,
      altKey: modifiers.alt || false,
      metaKey: modifiers.win || false,
      bubbles: true,
      cancelable: true,
    }));
  }

  const modFlags: Record<string, boolean> = {};
  for (const m of mods) {
    const ml = m.toLowerCase();
    modFlags[ml] = true;
    const code = MODIFIER_CODE[ml] || ml;
    const key = ml === 'win' ? 'Meta' : ml.charAt(0).toUpperCase() + ml.slice(1);
    dispatch('keydown', key, code, modFlags);
  }

  for (const m of mains) {
    const ml = m.toLowerCase();
    const code = KEY_CODE[ml] || ml;
    dispatch('keydown', ml, code, modFlags);
    dispatch('keyup', ml, code, modFlags);
  }

  for (const m of [...mods].reverse()) {
    const ml = m.toLowerCase();
    modFlags[ml] = false;
    const code = MODIFIER_CODE[ml] || ml;
    const key = ml === 'win' ? 'Meta' : ml.charAt(0).toUpperCase() + ml.slice(1);
    dispatch('keyup', key, code, modFlags);
  }
}
