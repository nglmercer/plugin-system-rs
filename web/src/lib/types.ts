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
