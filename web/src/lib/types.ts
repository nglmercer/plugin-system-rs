export interface DeviceInfo {
  id: string;
  name: string;
  key_count: number;
  is_virtual: boolean;
}

export interface Profile {
  id: string;
  name: string;
  pages: Page[];
}

export interface Page {
  buttons: ButtonBinding[];
}

export interface ButtonBinding {
  action_id: string | null;
  settings: Record<string, any>;
  label: string;
  icon: string;
}

export interface StreamEvent {
  type: string;
  [key: string]: any;
}

export interface SystemStats {
  cpu_usage: number;
  memory_total: number;
  memory_used: number;
  memory_usage: number;
  swap_total: number;
  swap_used: number;
  load_avg: [number, number, number];
  uptime: number;
  process_count: number;
}

export interface PluginData {
  name: string;
  version: string;
  interfaces: string[];
  data: Record<string, any>;
}

export type WidgetType = 'system-monitor' | 'clock' | 'actions';

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  colSpan: number;
  rowSpan: number;
  settings: Record<string, any>;
}

export interface DashboardLayout {
  widgets: WidgetConfig[];
  columns: number;
}
