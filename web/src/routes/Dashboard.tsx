import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchDevices, simulateButtonPress } from '../lib/api';
import { DeviceInfo, StreamEvent } from '../lib/types';

interface DashboardProps {
  events: StreamEvent[];
}

export function Dashboard({ events }: DashboardProps) {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [pressedButtons, setPressedButtons] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadDevices();
  }, []);

  async function loadDevices() {
    const devices = await fetchDevices();
    setDevices(devices);
  }

  async function handleButtonPress(deviceId: string, index: number) {
    const key = `${deviceId}-${index}`;
    setPressedButtons(prev => new Set(prev).add(key));

    await simulateButtonPress(deviceId, index);

    setTimeout(() => {
      setPressedButtons(prev => {
        const next = new Set(prev);
        next.delete(key);
        return next;
      });
    }, 200);
  }

  return h('div', { class: 'dashboard' },
    h('h2', null, 'Dashboard'),

    h('div', { class: 'devices' },
      devices.map(device =>
        h('div', { class: 'device', key: device.id },
          h('h3', null, `${device.name} (${device.key_count} keys)`),
          h('div', { class: 'button-grid' },
            Array.from({ length: device.key_count }, (_, i) => {
              const key = `${device.id}-${i}`;
              const isPressed = pressedButtons.has(key);
              return h('button', {
                key: i,
                class: `deck-button ${isPressed ? 'pressed' : ''}`,
                onClick: () => handleButtonPress(device.id, i),
              }, `${i + 1}`);
            })
          )
        )
      )
    ),

    h('div', { class: 'events-panel' },
      h('h3', null, 'Recent Events'),
      h('div', { class: 'events-list' },
        events.slice(0, 10).map((event, i) =>
          h('div', { class: 'event-item', key: i },
            h('span', { class: 'event-type' }, event.type || 'unknown'),
            h('pre', null, JSON.stringify(event, null, 2))
          )
        )
      )
    )
  );
}
