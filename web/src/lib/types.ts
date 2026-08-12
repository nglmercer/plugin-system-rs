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
  cpu_model: string;
  cpu_cores: number;
  memory_total: number;
  memory_used: number;
  memory_usage: number;
  swap_total: number;
  swap_used: number;
  load_avg: [number, number, number];
  uptime: number;
  process_count: number;
  thread_count: number;
}

export interface PluginData {
  name: string;
  version: string;
  interfaces: string[];
  data: Record<string, any>;
}

export interface PluginStatus {
  name: string;
  path: string;
  loaded: boolean;
  enabled: boolean;
  version: string;
}

/**
 * A widget's type id.
 *
 * Deliberately a plain string rather than a union of the built-ins. The set of
 * valid types is decided at runtime by the registry — a plugin can add one —
 * so a compile-time union would have been a list that is wrong by construction
 * the moment anything is contributed. `getWidget` is what tells you whether a
 * type is real.
 */
export type WidgetType = string;

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  /**
   * Footprint in grid cells. Named `colSpan`/`rowSpan` for continuity with
   * the flow layout these replaced; in the deck they are simply width and
   * height.
   */
  colSpan: number;
  rowSpan: number;
  /**
   * Explicit position, in cell units. Optional so a layout saved before the
   * deck existed still loads — `normalizeLayout` assigns coordinates to any
   * widget missing them, once.
   */
  x?: number;
  y?: number;
  page?: number;
  settings: Record<string, any>;
}

export type WidgetVariant = string;

export interface DashboardLayout {
  widgets: WidgetConfig[];
  /** Cells across one page. */
  columns: number;
  /** Cells down one page. Absent in pre-deck layouts. */
  rows?: number;
  /** Cell width / height. 1 gives square keys like deck hardware. */
  aspect?: number;
}

export interface CSSCustomProperties {
  [key: `--${string}`]: string | number;
}
