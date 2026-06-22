import { h } from "preact";
import { useState } from "preact/hooks";
import { WidgetConfig, WIDGET_VARIANTS } from "../lib/types";
import { recordHotkey, resetHotkeyRecording } from "../lib/api";
import { WidgetContent } from "./WidgetContent";
import {
  FormField,
  FormInput,
  FormSelect,
  FormTextarea,
  KeyValueEditor,
  CollapsibleSection,
} from "./FormComponents";

interface WidgetWizardProps {
  widget: WidgetConfig;
  columns: number;
  onSave: (
    id: string,
    updates: {
      title?: string;
      colSpan?: number;
      settings?: Record<string, any>;
    },
  ) => void;
  onRemove: () => void;
  onClose: () => void;
}

export function WidgetWizard({
  widget,
  columns,
  onSave,
  onRemove,
  onClose,
}: WidgetWizardProps) {
  const [step, setStep] = useState(0);
  const [title, setTitle] = useState(widget.title);
  const [colSpan, setColSpan] = useState(widget.colSpan);
  const [settings, setSettings] = useState({ ...widget.settings });
  const [variant, setVariant] = useState<string>(
    widget.settings.variant || "compact",
  );
  const totalSteps = 4;

  function handleNext() {
    if (step < totalSteps - 1) setStep(step + 1);
  }
  function handleBack() {
    if (step > 0) setStep(step - 1);
  }
  function handleApply() {
    onSave(widget.id, { title, colSpan, settings: { ...settings, variant } });
  }

  function updateSetting(key: string, value: any) {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }

  return h(
    "div",
    { class: "wizard-overlay", onClick: onClose },
    h(
      "div",
      { class: "wizard-modal", onClick: (e: Event) => e.stopPropagation() },
      h(
        "div",
        { class: "wizard-header" },
        h("div", { class: "wizard-title" }, `Edit: ${widget.type}`),
        h("button", { class: "picker-close", onClick: onClose }, "\u2715"),
      ),
      h(
        "div",
        { class: "wizard-steps" },
        ["General", "Config", "Style", "Apply"].map((label, i) =>
          h(
            "div",
            {
              class: `wizard-step-indicator ${i === step ? "active" : i < step ? "done" : ""}`,
              key: label,
              onClick: () => setStep(i),
            },
            h("div", { class: "wizard-step-circle" }, label[0]),
            h("div", { class: "wizard-step-label" }, label),
          ),
        ),
      ),
      h(
        "div",
        { class: "wizard-body" },
        step === 0 &&
          h(WizardGeneral, {
            title,
            colSpan,
            columns,
            onChangeTitle: setTitle,
            onChangeColSpan: setColSpan,
          }),
        step === 1 &&
          h(WizardConfig, {
            widget,
            settings,
            onChange: setSettings,
            updateSetting,
          }),
        step === 2 && h(WizardStyle, { widget, variant, onChange: setVariant }),
        step === 3 &&
          h(WizardConfirm, {
            widget,
            title,
            colSpan,
            settings,
            variant,
            onApply: handleApply,
            onRemove,
          }),
      ),
      h(
        "div",
        { class: "wizard-footer" },
        step > 0 &&
          h(
            "button",
            { class: "wizard-btn back", onClick: handleBack },
            "Back",
          ),
        h("div", { class: "wizard-footer-spacer" }),
        step < totalSteps - 1
          ? h(
              "button",
              { class: "wizard-btn next", onClick: handleNext },
              "Next",
            )
          : h(
              "button",
              { class: "wizard-btn apply", onClick: handleApply },
              "Save & Close",
            ),
      ),
    ),
  );
}

/* ── General Step ─────────────────────────────────────── */

function WizardGeneral({
  title,
  colSpan,
  columns,
  onChangeTitle,
  onChangeColSpan,
}: {
  title: string;
  colSpan: number;
  columns: number;
  onChangeTitle: (t: string) => void;
  onChangeColSpan: (c: number) => void;
}) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "General Settings"),
    h(
      FormField,
      { label: "Widget Title" },
      h(FormInput, {
        value: title,
        placeholder: "Enter widget title...",
        onInput: onChangeTitle,
      }),
    ),
    h(
      FormField,
      { label: "Column Span", hint: `Grid has ${columns} columns` },
      h(FormInput, {
        type: "number",
        value: String(colSpan),
        onInput: (v) => onChangeColSpan(parseInt(v) || 1),
      }),
    ),
  );
}

/* ── Config Step ──────────────────────────────────────── */

function WizardConfig({
  widget,
  settings,
  onChange,
  updateSetting,
}: {
  widget: WidgetConfig;
  settings: Record<string, any>;
  onChange: (s: Record<string, any>) => void;
  updateSetting: (key: string, value: any) => void;
}) {
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

/* ── Interval Field (reusable) ────────────────────────── */

function IntervalField({
  value,
  min = 500,
  onChange,
}: {
  value: number;
  min?: number;
  onChange: (v: number) => void;
}) {
  return h(
    FormField,
    { label: "Refresh Interval", hint: "ms" },
    h(FormInput, {
      type: "number",
      value: String(value),
      onInput: (v) => onChange(parseInt(v) || min),
    }),
  );
}

/* ── Fetch Config ─────────────────────────────────────── */

function FetchConfig({
  settings,
  updateSetting,
}: {
  settings: Record<string, any>;
  updateSetting: (key: string, value: any) => void;
}) {
  const method = settings.method || "GET";
  const hasBody = ["POST", "PUT", "PATCH"].includes(method);

  return h(
    "div",
    { class: "wizard-step-content" },

    h(
      FormField,
      { label: "URL" },
      h(FormInput, {
        value: settings.url || "",
        placeholder: "https://api.example.com/data",
        onInput: (v) => updateSetting("url", v),
      }),
    ),

    h(
      "div",
      { class: "wizard-field-row" },
      h(
        FormField,
        { label: "Fetch Mode" },
        h(FormSelect, {
          value: settings.mode || "proxy",
          options: [
            { value: "local", label: "Local (Browser)" },
            { value: "proxy", label: "Proxy (Backend)" },
          ],
          onChange: (v) => updateSetting("mode", v),
        }),
      ),
      h(
        FormField,
        { label: "HTTP Method" },
        h(FormSelect, {
          value: method,
          options: [
            { value: "GET", label: "GET" },
            { value: "POST", label: "POST" },
            { value: "PUT", label: "PUT" },
            { value: "DELETE", label: "DELETE" },
            { value: "PATCH", label: "PATCH" },
          ],
          onChange: (v) => updateSetting("method", v),
        }),
      ),
    ),

    h(
      "div",
      { class: "wizard-field-row" },
      h(
        FormField,
        { label: "Fetch Mode" },
        h(FormSelect, {
          value: settings.fetchMode || "auto",
          options: [
            { value: "once", label: "Once (manual refresh)" },
            { value: "auto", label: "Auto (interval)" },
          ],
          onChange: (v) => updateSetting("fetchMode", v),
        }),
      ),
      settings.fetchMode !== "once" &&
        h(
          FormField,
          { label: "Interval", hint: "seconds" },
          h(FormInput, {
            type: "number",
            value: String(settings.intervalSec || 30),
            onInput: (v) => updateSetting("intervalSec", parseInt(v) || 30),
          }),
        ),
    ),

    h(
      CollapsibleSection,
      { title: "Headers", defaultOpen: !!(settings.headers && settings.headers.trim()) },
      h(KeyValueEditor, {
        value: settings.headers || "",
        placeholder: { key: "Authorization", value: "Bearer token" },
        onChange: (v) => updateSetting("headers", v),
      }),
    ),

    hasBody &&
      h(
        CollapsibleSection,
        { title: "Request Body", defaultOpen: true },
        h(
          "div",
          { class: "wizard-step-content" },
          h(
            FormField,
            { label: "Body Type" },
            h(FormSelect, {
              value: settings.bodyType || "json",
              options: [
                { value: "json", label: "JSON" },
                { value: "raw", label: "Raw Text" },
                { value: "form", label: "Form Data" },
              ],
              onChange: (v) => updateSetting("bodyType", v),
            }),
          ),
          settings.bodyType === "json" &&
            h(
              FormField,
              { label: "JSON Body" },
              h(FormTextarea, {
                value: settings.body || "",
                placeholder: '{\n  "key": "value"\n}',
                rows: 5,
                onInput: (v) => updateSetting("body", v),
              }),
            ),
          settings.bodyType === "raw" &&
            h(
              FormField,
              { label: "Raw Body" },
              h(FormTextarea, {
                value: settings.body || "",
                placeholder: "Raw request body...",
                rows: 5,
                onInput: (v) => updateSetting("body", v),
              }),
            ),
          settings.bodyType === "form" &&
            h(
              FormField,
              { label: "Form Fields" },
              h(KeyValueEditor, {
                value: settings.body || "",
                placeholder: { key: "field", value: "value" },
                onChange: (v) => updateSetting("body", v),
              }),
            ),
        ),
      ),
  );
}

/* ── Hotkey Recorder ──────────────────────────────────── */

function HotkeyRecorder({
  currentKeys,
  onChange,
}: {
  currentKeys: string;
  onChange: (keys: string) => void;
}) {
  const [recording, setRecording] = useState(false);
  const [selectedKeys, setSelectedKeys] = useState<string[]>(
    currentKeys ? currentKeys.split("+").filter(Boolean) : []
  );
  const [showPicker, setShowPicker] = useState(false);

  const MODIFIERS = ["ctrl", "shift", "alt", "win"];
  const MODIFIER_LABELS: Record<string, string> = {
    ctrl: "Ctrl",
    shift: "Shift",
    alt: "Alt",
    win: "Win",
  };
  const LETTER_KEYS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");
  const NUMBER_KEYS = "0123456789".split("");
  const FUNCTION_KEYS = Array.from({ length: 12 }, (_, i) => `f${i + 1}`);
  const SPECIAL_KEYS = [
    { key: "space", label: "Space" },
    { key: "enter", label: "Enter" },
    { key: "tab", label: "Tab" },
    { key: "escape", label: "Esc" },
    { key: "backspace", label: "Backspace" },
    { key: "delete", label: "Del" },
    { key: "home", label: "Home" },
    { key: "end", label: "End" },
    { key: "pageup", label: "PgUp" },
    { key: "pagedown", label: "PgDn" },
    { key: "up", label: "\u2191" },
    { key: "down", label: "\u2193" },
    { key: "left", label: "\u2190" },
    { key: "right", label: "\u2192" },
  ];

  function toggleKey(key: string) {
    const lower = key.toLowerCase();
    setSelectedKeys((prev) =>
      prev.includes(lower)
        ? prev.filter((k) => k !== lower)
        : [...prev, lower]
    );
  }

  function removeKey(key: string) {
    setSelectedKeys((prev) => prev.filter((k) => k !== key));
  }

  function clearAll() {
    setSelectedKeys([]);
  }

  function applySelection() {
    if (selectedKeys.length > 0) {
      onChange(selectedKeys.join("+"));
      setShowPicker(false);
    }
  }

  async function startRecording() {
    setRecording(true);
    try {
      const combo = await recordHotkey(2000);
      if (combo) {
        setSelectedKeys(combo.split("+").filter(Boolean));
      }
    } catch (e) {
      if (e instanceof Error && e.message.includes("Already recording")) {
        await resetHotkeyRecording();
        try {
          const combo = await recordHotkey(2000);
          if (combo) {
            setSelectedKeys(combo.split("+").filter(Boolean));
          }
        } catch {}
      }
    }
    setRecording(false);
  }

  const combo = selectedKeys.join("+");

  return h(
    "div",
    { class: "wizard-field" },
    h("label", { class: "form-label" }, "Hotkey Combination"),
    h(
      "div",
      { class: "hotkey-display" },
      h("span", { class: "hotkey-keys" }, combo || "Not set"),
      h(
        "button",
        { class: "hotkey-record-btn", onClick: () => setShowPicker(!showPicker) },
        showPicker ? "Close" : "Select"
      ),
      h(
        "button",
        {
          class: `hotkey-record-btn ${recording ? "recording" : ""}`,
          onClick: recording ? () => {} : startRecording,
        },
        recording ? "..." : "Record"
      )
    ),
    showPicker &&
      h(
        "div",
        { class: "key-picker" },
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Selected:"),
          h(
            "div",
            { class: "key-picker-selected" },
            selectedKeys.length === 0
              ? h("span", { class: "key-picker-empty" }, "No keys selected")
              : selectedKeys.map((key) =>
                  h(
                    "span",
                    { class: "key-picker-chip", key, onClick: () => removeKey(key) },
                    key,
                    h("span", { class: "key-picker-chip-x" }, "\u00D7")
                  )
                )
          ),
          selectedKeys.length > 0 &&
            h(
              "div",
              { class: "key-picker-actions" },
              h("button", { class: "key-picker-clear", onClick: clearAll }, "Clear"),
              h("button", { class: "key-picker-apply", onClick: applySelection }, "Apply")
            )
        ),
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Modifiers:"),
          h(
            "div",
            { class: "key-picker-modifiers" },
            MODIFIERS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-mod ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                MODIFIER_LABELS[key]
              )
            )
          )
        ),
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Letters:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-letters" },
            LETTER_KEYS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key.toLowerCase()) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                key
              )
            )
          )
        ),
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Numbers:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-numbers" },
            NUMBER_KEYS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                key
              )
            )
          )
        ),
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Function Keys:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-functions" },
            FUNCTION_KEYS.map((key) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                key.toUpperCase()
              )
            )
          )
        ),
        h(
          "div",
          { class: "key-picker-section" },
          h("div", { class: "key-picker-label" }, "Special Keys:"),
          h(
            "div",
            { class: "key-picker-grid key-picker-special" },
            SPECIAL_KEYS.map(({ key, label }) =>
              h(
                "button",
                {
                  key,
                  class: `key-picker-key ${selectedKeys.includes(key) ? "active" : ""}`,
                  onClick: () => toggleKey(key),
                },
                label
              )
            )
          )
        )
      )
  );
}

/* ── Style Step ───────────────────────────────────────── */

function WizardStyle({
  widget,
  variant,
  onChange,
}: {
  widget: WidgetConfig;
  variant: string;
  onChange: (v: string) => void;
}) {
  const entries = WIDGET_VARIANTS.find((e) => e.type === widget.type);
  if (!entries) return null;

  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Style Variant"),
    h("p", { class: "wizard-step-desc" }, "Choose how this widget displays"),
    h(
      "div",
      { class: "variant-grid" },
      entries.variants.map((v) =>
        h(
          "button",
          {
            class: `variant-card ${variant === v.value ? "selected" : ""}`,
            key: v.value,
            onClick: () => onChange(v.value),
          },
          h(
            "div",
            { class: "variant-card-preview" },
            h(VariantPreview, { type: widget.type, variant: v.value }),
          ),
          h(
            "div",
            { class: "variant-card-info" },
            h("div", { class: "variant-card-label" }, v.label),
            h("div", { class: "variant-card-desc" }, v.description),
          ),
        ),
      ),
    ),
  );
}

/* ── Variant Preview ──────────────────────────────────── */

function VariantPreview({ type, variant }: { type: string; variant: string }) {
  switch (type) {
    case "system-monitor":
      switch (variant) {
        case "minimal":
          return h(
            "div",
            { class: "variant-preview sysmon-minimal" },
            h("div", null, "42% CPU"),
            h("div", null, "56% RAM"),
          );
        case "compact":
          return h(
            "div",
            { class: "variant-preview sysmon-compact" },
            h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "42%", background: "#4caf50" } })),
            h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "56%", background: "#2196f3" } })),
          );
        case "detailed":
          return h(
            "div",
            { class: "variant-preview sysmon-detailed" },
            h("div", { class: "mini-grid" }, h("div", null, "42%"), h("div", null, "56%"), h("div", null, "1.2"), h("div", null, "2d")),
          );
      }
    case "clock":
      switch (variant) {
        case "simple":
          return h("div", { class: "variant-preview clock-simple" }, "14:30");
        case "digital":
          return h("div", { class: "variant-preview clock-digital" }, "14:30", h("div", { class: "mini-sec" }, "15"), h("div", { class: "mini-date" }, "Mon"));
        case "detailed":
          return h("div", { class: "variant-preview clock-detailed" }, "14:30:15", h("div", { class: "mini-date" }, "Monday, Jun 10"));
      }
    case "volume-master":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview vol-minimal" }, h("div", null, "75%"), h("div", { class: "mini-btn" }, "MUTE"));
        case "compact":
          return h("div", { class: "variant-preview vol-compact" }, h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })), h("div", null, "Speaker"));
        case "detailed":
          return h("div", { class: "variant-preview vol-detailed" }, h("div", null, "75%"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })), h("div", { class: "mini-apps" }, "Apps: 2"));
      }
    case "volume-apps":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview volapps-minimal" }, h("div", null, "3 apps"), h("div", { class: "mini-list" }, "Firefox, Spotify"));
        case "compact":
          return h("div", { class: "variant-preview volapps-compact" }, h("div", null, "Firefox"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "60%", background: "#4caf50" } })));
        case "detailed":
          return h("div", { class: "variant-preview volapps-detailed" }, h("div", null, "Firefox (PID: 1234)"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "60%", background: "#4caf50" } })), h("div", null, "60%"));
      }
    case "obs-control":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview obs-minimal" }, h("div", { class: "mini-row" }, h("div", { class: "mini-dot green" }), h("span", null, "Connected")), h("div", { class: "mini-row" }, h("div", { class: "mini-dot red" }), h("span", null, "Stream")));
        case "compact":
          return h("div", { class: "variant-preview obs-compact" }, h("div", null, "Scene 1"), h("div", { class: "mini-btns" }, h("div", { class: "mini-btn" }, "STR"), h("div", { class: "mini-btn" }, "REC"), h("div", { class: "mini-btn" }, "VC")));
        case "detailed":
          return h("div", { class: "variant-preview obs-detailed" }, h("div", { class: "mini-btns" }, h("div", { class: "mini-btn active" }, "Stream"), h("div", { class: "mini-btn" }, "Record")), h("div", { class: "mini-grid" }, h("div", null, "CPU"), h("div", null, "FPS")));
      }
    case "obs-scenes":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview obscene-minimal" }, h("div", null, "Scene 1"), h("div", { class: "mini-grid" }, h("div", { class: "mini-btn active" }, "S1"), h("div", { class: "mini-btn" }, "S2")));
        case "compact":
          return h("div", { class: "variant-preview obscene-compact" }, h("div", { class: "mini-list" }, h("div", { class: "mini-btn active" }, "Scene 1"), h("div", { class: "mini-btn" }, "Scene 2")));
        case "detailed":
          return h("div", { class: "variant-preview obscene-detailed" }, h("div", { class: "mini-list" }, h("div", { class: "mini-btn active" }, "Scene 1"), h("div", { class: "mini-btn" }, "Scene 2")), h("div", { class: "mini-btns" }, h("div", { class: "mini-btn" }, "Fade")));
      }
    case "obs-inputs":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview obsinput-minimal" }, h("div", null, "3 inputs"), h("div", { class: "mini-list" }, h("div", { class: "mini-row" }, h("span", null, "Mic"), h("div", { class: "mini-btn" }, "M"))));
        case "compact":
          return h("div", { class: "variant-preview obsinput-compact" }, h("div", null, "Mic"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })));
        case "detailed":
          return h("div", { class: "variant-preview obsinput-detailed" }, h("div", null, "Mic (audio)"), h("div", { class: "mini-bar" }, h("div", { class: "mini-bar-fill", style: { width: "75%", background: "#4caf50" } })), h("div", null, "75%"));
      }
    case "fetch":
      switch (variant) {
        case "minimal":
          return h("div", { class: "variant-preview fetch-minimal" }, h("div", { class: "mini-status ok" }, "200"));
        case "compact":
          return h("div", { class: "variant-preview fetch-compact" }, h("div", { class: "mini-url" }, "api.ex..."), h("div", { class: "mini-data" }, '{"id":1...}'));
        case "detailed":
          return h("div", { class: "variant-preview fetch-detailed" }, h("div", { class: "mini-url" }, "https://api.example.com/v1"), h("div", { class: "mini-json" }, '{\n  "status": "ok",\n  "data": [...]\n}'));
      }
    default:
      return h(
        "div",
        { class: "variant-preview simple-preview" },
        h("div", { class: variant === "compact" ? "preview-btn-sm" : "preview-btn-lg" }, "Action"),
      );
  }
}

/* ── Confirm Step ─────────────────────────────────────── */

function WizardConfirm({
  widget,
  title,
  colSpan,
  settings,
  variant,
  onApply,
  onRemove,
}: {
  widget: WidgetConfig;
  title: string;
  colSpan: number;
  settings: Record<string, any>;
  variant: string;
  onApply: () => void;
  onRemove: () => void;
}) {
  return h(
    "div",
    { class: "wizard-step-content" },
    h("h3", { class: "wizard-step-heading" }, "Confirm Changes"),
    h("p", { class: "wizard-step-desc" }, "Review your widget configuration before saving"),
    h(
      "div",
      { class: "confirm-details" },
      h("div", { class: "confirm-row" }, h("span", null, "Title"), h("span", null, title)),
      h("div", { class: "confirm-row" }, h("span", null, "Span"), h("span", null, `${colSpan} column${colSpan > 1 ? "s" : ""}`)),
      h("div", { class: "confirm-row" }, h("span", null, "Variant"), h("span", null, variant)),
      ...Object.entries(settings)
        .filter(([k]) => k !== "variant")
        .map(([k, v]) =>
          h(
            "div",
            { class: "confirm-row", key: k },
            h("span", null, k),
            h("span", null, String(v).substring(0, 60)),
          ),
        ),
    ),
    h(
      "div",
      { class: "confirm-preview" },
      h("div", { class: "wizard-step-heading", style: "font-size:0.8rem;color:#888;margin-bottom:0.5rem" }, "Preview"),
      h(
        "div",
        { class: "preview-frame" },
        h(WidgetContent, {
          widget: { ...widget, title, colSpan, settings: { ...settings, variant } },
        }),
      ),
    ),
    h("button", { class: "wizard-remove-btn", onClick: onRemove }, "Delete Widget"),
  );
}
