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
export async function createProfile(name) {
    const res = await fetch(`${API_BASE}/profiles`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
    });
    const data = await res.json();
    return data.data;
}
export async function deleteProfile(id) {
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
export async function simulateButtonPress(deviceId, buttonIndex) {
    const res = await fetch(`${API_BASE}/devices/${deviceId}/press/${buttonIndex}`, {
        method: 'POST',
    });
    const data = await res.json();
    return data;
}
