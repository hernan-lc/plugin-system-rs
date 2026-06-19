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

export type WidgetType =
  | "system-monitor"
  | "clock"
  | "send-hotkey"
  | "open-url"
  | "type-text"
  | "volume-master"
  | "volume-apps"
  | "obs-control"
  | "obs-scenes"
  | "obs-inputs";

export interface WidgetConfig {
  id: string;
  type: WidgetType;
  title: string;
  colSpan: number;
  rowSpan: number;
  settings: Record<string, any>;
}

export type WidgetVariant = string;

export const WIDGET_VARIANTS: {
  type: WidgetType;
  variants: { value: string; label: string; description: string }[];
}[] = [
  {
    type: "system-monitor",
    variants: [
      {
        value: "minimal",
        label: "Minimal",
        description: "Just CPU% and RAM% numbers",
      },
      {
        value: "compact",
        label: "Compact",
        description: "CPU + RAM bars with load",
      },
      {
        value: "detailed",
        label: "Detailed",
        description: "Full stats with swap, cores, uptime",
      },
    ],
  },
  {
    type: "clock",
    variants: [
      {
        value: "simple",
        label: "Simple",
        description: "Just HH:MM, no seconds",
      },
      {
        value: "digital",
        label: "Digital",
        description: "HH:MM + seconds + date",
      },
      {
        value: "detailed",
        label: "Detailed",
        description: "Full date with day name",
      },
    ],
  },

  {
    type: "send-hotkey",
    variants: [
      {
        value: "compact",
        label: "Mini",
        description: "Small button with hotkey display",
      },
      {
        value: "detailed",
        label: "Full",
        description: "Large button with hotkey + description",
      },
    ],
  },
  {
    type: "open-url",
    variants: [
      { value: "compact", label: "Mini", description: "Small button with URL" },
      {
        value: "detailed",
        label: "Full",
        description: "Large button with URL preview",
      },
    ],
  },
  {
    type: "type-text",
    variants: [
      {
        value: "compact",
        label: "Mini",
        description: "Small button with text preview",
      },
      {
        value: "detailed",
        label: "Full",
        description: "Large button with full text",
      },
    ],
  },
  {
    type: "volume-master",
    variants: [
      {
        value: "minimal",
        label: "Minimal",
        description: "Just volume % and mute button",
      },
      {
        value: "compact",
        label: "Compact",
        description: "Slider with device name and app list",
      },
      {
        value: "detailed",
        label: "Detailed",
        description: "Full controls with per-app volume",
      },
    ],
  },
  {
    type: "volume-apps",
    variants: [
      {
        value: "minimal",
        label: "Minimal",
        description: "App count + mini list",
      },
      { value: "compact", label: "Compact", description: "List with sliders" },
      {
        value: "detailed",
        label: "Detailed",
        description: "Full per-app controls",
      },
    ],
  },
  {
    type: "obs-control",
    variants: [
      {
        value: "minimal",
        label: "Minimal",
        description: "Status dots for stream/record",
      },
      {
        value: "compact",
        label: "Compact",
        description: "Current scene + toggle buttons",
      },
      {
        value: "detailed",
        label: "Detailed",
        description: "Full controls + stats + transitions",
      },
    ],
  },
  {
    type: "obs-scenes",
    variants: [
      {
        value: "minimal",
        label: "Minimal",
        description: "Current scene + grid buttons",
      },
      {
        value: "compact",
        label: "Compact",
        description: "Scene list with active highlight",
      },
      {
        value: "detailed",
        label: "Detailed",
        description: "Scenes + transitions + source toggles",
      },
    ],
  },
  {
    type: "obs-inputs",
    variants: [
      {
        value: "minimal",
        label: "Minimal",
        description: "Input count + mute toggles",
      },
      {
        value: "compact",
        label: "Compact",
        description: "List with sliders and mute",
      },
      {
        value: "detailed",
        label: "Detailed",
        description: "Full input controls with kind info",
      },
    ],
  },
];

export interface WizardStep {
  id: string;
  label: string;
  icon: string;
}

export const WIZARD_STEPS: WizardStep[] = [
  { id: "general", label: "General", icon: "G" },
  { id: "config", label: "Config", icon: "C" },
  { id: "style", label: "Style", icon: "S" },
  { id: "confirm", label: "Apply", icon: "✓" },
];

export interface DashboardLayout {
  widgets: WidgetConfig[];
  columns: number;
}
