import { WidgetType, WidgetConfig } from "../lib/types";

export const WIDGET_CATALOG: {
  type: WidgetType;
  label: string;
  icon: string;
  description: string;
  defaultColSpan: number;
  defaultRowSpan: number;
}[] = [
  {
    type: "system-monitor",
    label: "System Monitor",
    icon: "%",
    description: "CPU, Memory, Load, Uptime",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "clock",
    label: "Clock",
    icon: "T",
    description: "Current time & date",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "send-hotkey",
    label: "Send Hotkey",
    icon: "H",
    description: "Trigger keyboard hotkey",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "open-url",
    label: "Open URL",
    icon: "U",
    description: "Open a URL in browser",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "type-text",
    label: "Type Text",
    icon: "A",
    description: "Type text string",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "volume-master",
    label: "Volume Control",
    icon: "V",
    description: "Master volume slider",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "volume-apps",
    label: "App Volume",
    icon: "A",
    description: "Per-app volume control",
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
];

export function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

export function getDefaultVariant(type: WidgetType): string {
  const variants: Record<string, string> = {
    "system-monitor": "compact",
    clock: "digital",
    "send-hotkey": "compact",
    "open-url": "compact",
    "type-text": "compact",
    "volume-master": "compact",
    "volume-apps": "compact",
  };
  return variants[type] || "compact";
}

export function getDefaultSettings(type: WidgetType): Record<string, any> {
  const settings: Record<string, any> = { variant: getDefaultVariant(type) };
  switch (type) {
    case "system-monitor":
      settings.refreshInterval = 2000;
      break;
    case "send-hotkey":
      settings.keys = "ctrl+c";
      break;
    case "open-url":
      settings.url = "https://example.com";
      break;
    case "type-text":
      settings.text = "Hello!";
      break;
    case "volume-master":
      settings.refreshInterval = 2000;
      break;
    case "volume-apps":
      settings.refreshInterval = 2000;
      break;
  }
  return settings;
}

export function buildDefaultWidget(type: WidgetType): WidgetConfig {
  const catalog = WIDGET_CATALOG.find((w) => w.type === type)!;
  return {
    id: generateId(),
    type,
    title: catalog.label,
    colSpan: catalog.defaultColSpan,
    rowSpan: catalog.defaultRowSpan,
    settings: getDefaultSettings(type),
  };
}
