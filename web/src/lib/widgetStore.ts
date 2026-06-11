import { WidgetConfig, DashboardLayout } from './types';

const STORAGE_KEY = 'streamdeck-dashboard-layout';

const DEFAULT_LAYOUT: DashboardLayout = {
  columns: 12,
  widgets: [
    {
      id: 'device-1',
      type: 'button-grid',
      title: 'Virtual StreamDeck',
      colSpan: 6,
      rowSpan: 1,
      settings: { deviceId: 'virtual' },
    },
    {
      id: 'events-1',
      type: 'event-log',
      title: 'Recent Events',
      colSpan: 6,
      rowSpan: 1,
      settings: { maxEvents: 20 },
    },
    {
      id: 'sysmon-1',
      type: 'system-monitor',
      title: 'System Monitor',
      colSpan: 4,
      rowSpan: 1,
      settings: { refreshInterval: 2000 },
    },
    {
      id: 'clock-1',
      type: 'clock',
      title: 'Clock',
      colSpan: 4,
      rowSpan: 1,
      settings: { format: 'digital' },
    },
    {
      id: 'actions-1',
      type: 'actions',
      title: 'Quick Actions',
      colSpan: 4,
      rowSpan: 1,
      settings: {},
    },
  ],
};

export function loadLayout(): DashboardLayout {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (e) {
    console.error('Failed to load layout:', e);
  }
  return DEFAULT_LAYOUT;
}

export function saveLayout(layout: DashboardLayout): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch (e) {
    console.error('Failed to save layout:', e);
  }
}

export function generateWidgetId(): string {
  return `widget-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

export function createWidget(type: WidgetConfig['type'], title: string): WidgetConfig {
  const defaults: Record<WidgetType, Partial<WidgetConfig>> = {
    'button-grid': { colSpan: 6, rowSpan: 1, settings: { deviceId: 'virtual' } },
    'event-log': { colSpan: 6, rowSpan: 1, settings: { maxEvents: 20 } },
    'system-monitor': { colSpan: 4, rowSpan: 1, settings: { refreshInterval: 2000 } },
    'clock': { colSpan: 4, rowSpan: 1, settings: { format: 'digital' } },
    'actions': { colSpan: 4, rowSpan: 1, settings: {} },
  };

  const config = defaults[type];
  return {
    id: generateWidgetId(),
    type,
    title,
    colSpan: config.colSpan || 4,
    rowSpan: config.rowSpan || 1,
    settings: config.settings || {},
  };
}

type WidgetType = WidgetConfig['type'];
