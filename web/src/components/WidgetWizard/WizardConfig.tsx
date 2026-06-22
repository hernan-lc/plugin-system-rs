import { h } from "preact";
import { WidgetConfig } from "../../lib/types";
import { FormField, FormInput, FormTextarea } from "../FormComponents";
import { HotkeyRecorder } from "./HotkeyRecorder";
import { IntervalField } from "./IntervalField";
import { FetchConfig } from "./FetchConfig";

interface WizardConfigProps {
  widget: WidgetConfig;
  settings: Record<string, any>;
  onChange: (s: Record<string, any>) => void;
  updateSetting: (key: string, value: any) => void;
}

export function WizardConfig({
  widget,
  settings,
  onChange,
  updateSetting,
}: WizardConfigProps) {
  const set = (key: string, value: any) => updateSetting(key, value);

  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Widget Configuration"),

    widget.type === "send-hotkey" &&
      h(HotkeyRecorder, {
        currentKeys: settings.keys || "",
        onChange: (keys) => set("keys", keys),
      }),

    widget.type === "open-url" &&
      h(
        FormField,
        { label: "URL" },
        h(FormInput, {
          value: settings.url || "",
          placeholder: "https://example.com",
          onInput: (v) => set("url", v),
        }),
      ),

    widget.type === "type-text" &&
      h(
        FormField,
        { label: "Text" },
        h(FormTextarea, {
          value: settings.text || "",
          placeholder: "Text to type...",
          onInput: (v) => set("text", v),
        }),
      ),

    widget.type === "system-monitor" &&
      h(IntervalField, {
        value: settings.refreshInterval || 2000,
        min: 500,
        onChange: (v) => set("refreshInterval", v),
      }),

    widget.type === "volume-master" &&
      h(IntervalField, {
        value: settings.refreshInterval || 2000,
        min: 500,
        onChange: (v) => set("refreshInterval", v),
      }),

    widget.type === "volume-apps" &&
      h(IntervalField, {
        value: settings.refreshInterval || 2000,
        min: 500,
        onChange: (v) => set("refreshInterval", v),
      }),

    widget.type === "obs-control" &&
      h(
        "div",
        { class: "wizard-step-content" },
        h(
          FormField,
          { label: "OBS Host" },
          h(FormInput, {
            value: settings.host || "127.0.0.1",
            placeholder: "127.0.0.1",
            onInput: (v) => set("host", v),
          }),
        ),
        h(
          "div",
          { class: "wizard-field-row" },
          h(
            FormField,
            { label: "Port" },
            h(FormInput, {
              type: "number",
              value: String(settings.port || 4455),
              onInput: (v) => set("port", parseInt(v) || 4455),
            }),
          ),
          h(
            FormField,
            { label: "Password" },
            h(FormInput, {
              type: "password",
              value: settings.password || "",
              placeholder: "OBS WebSocket password",
              onInput: (v) => set("password", v),
            }),
          ),
        ),
        h(IntervalField, {
          value: settings.refreshInterval || 2000,
          min: 500,
          onChange: (v) => set("refreshInterval", v),
        }),
      ),

    widget.type === "obs-scenes" &&
      h(IntervalField, {
        value: settings.refreshInterval || 2000,
        min: 500,
        onChange: (v) => set("refreshInterval", v),
      }),

    widget.type === "obs-inputs" &&
      h(IntervalField, {
        value: settings.refreshInterval || 2000,
        min: 500,
        onChange: (v) => set("refreshInterval", v),
      }),

    widget.type === "fetch" &&
      h(FetchConfig, { settings, updateSetting }),
  );
}
