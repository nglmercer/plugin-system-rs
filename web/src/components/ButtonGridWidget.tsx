import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import { fetchDevices, simulateButtonPress } from '../lib/api';
import { DeviceInfo } from '../lib/types';

interface ButtonGridProps {
  settings: Record<string, any>;
}

export function ButtonGridWidget({ settings }: ButtonGridProps) {
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

  return h('div', { class: 'widget-button-grid' },
    devices.map(device =>
      h('div', { class: 'device-section', key: device.id },
        h('div', { class: 'device-header' },
          h('span', { class: 'device-name' }, device.name),
          h('span', { class: 'device-keys' }, `${device.key_count} keys`)
        ),
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
  );
}
