import { WidgetType, WidgetConfig } from "../lib/types";

export const WIDGET_CATALOG: {
  type: WidgetType;
  label: string;
  icon: string;
  description: string;
  hasConfig: boolean;
  defaultColSpan: number;
  defaultRowSpan: number;
}[] = [
  {
    type: "system-monitor",
    label: "System Monitor",
    icon: "%",
    description: "CPU, Memory, Load, Uptime",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "clock",
    label: "Clock",
    icon: "T",
    description: "Current time & date",
    hasConfig: false,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "send-hotkey",
    label: "Send Hotkey",
    icon: "H",
    description: "Trigger keyboard hotkey",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "open-url",
    label: "Open URL",
    icon: "U",
    description: "Open a URL in browser",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "type-text",
    label: "Type Text",
    icon: "A",
    description: "Type text string",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "volume-master",
    label: "Volume Control",
    icon: "V",
    description: "Master volume slider",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "volume-apps",
    label: "App Volume",
    icon: "A",
    description: "Per-app volume control",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "obs-control",
    label: "OBS Control",
    icon: "O",
    description: "Stream, record, virtual cam",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "obs-scenes",
    label: "OBS Scenes",
    icon: "S",
    description: "Scene switcher",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "obs-inputs",
    label: "OBS Inputs",
    icon: "I",
    description: "Input volume & mute",
    hasConfig: true,
    defaultColSpan: 1,
    defaultRowSpan: 1,
  },
  {
    type: "fetch",
    label: "Fetch Data",
    icon: "F",
    description: "Fetch and display data",
    hasConfig: true,
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
    "obs-control": "compact",
    "obs-scenes": "compact",
    "obs-inputs": "compact",
    fetch: "compact",
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
    case "obs-control":
      settings.refreshInterval = 2000;
      settings.host = "127.0.0.1";
      settings.port = 4455;
      settings.password = "";
      break;
    case "obs-scenes":
      settings.refreshInterval = 2000;
      break;
    case "obs-inputs":
      settings.refreshInterval = 2000;
      break;
    case "fetch":
      settings.url = "";
      settings.mode = "proxy";
      settings.method = "GET";
      settings.refreshInterval = 30000;
      break;
  }
  return settings;
}

export function widgetHasConfig(type: WidgetType): boolean {
  const entry = WIDGET_CATALOG.find((w) => w.type === type);
  return entry?.hasConfig ?? true;
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
